//! MCP 服务器注册服务模块
//!
//! 提供 MCP 服务器注册信息的增删改查与健康检查能力。

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::process::Command;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::mcp_server::{
    CreateMcpServerInput, McpHealthResult, McpServerRecord, UpdateMcpServerInput,
};
use crate::services::audit::AuditService;

/// MCP 服务器注册服务。
pub struct McpServerService;

impl McpServerService {
    /// 列出所有未删除的 MCP 服务器。
    pub async fn list_mcp_servers(pool: &SqlitePool) -> Result<Vec<McpServerRecord>, AppError> {
        let items = sqlx::query_as::<_, McpServerRecord>(
            "SELECT id, name, transport, command, args_json, env_json, permission_scope, enabled, is_deleted, created_at, display_name, description, health_status, last_health_check, updated_at FROM mcp_server_registry WHERE is_deleted = 0 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 根据 ID 获取 MCP 服务器。
    pub async fn get_mcp_server(pool: &SqlitePool, id: &str) -> Result<McpServerRecord, AppError> {
        let item = sqlx::query_as::<_, McpServerRecord>(
            "SELECT id, name, transport, command, args_json, env_json, permission_scope, enabled, is_deleted, created_at, display_name, description, health_status, last_health_check, updated_at FROM mcp_server_registry WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MCP 服务器不存在：{id}")))?;

        Ok(item)
    }

    /// 创建 MCP 服务器。
    pub async fn create_mcp_server(
        pool: &SqlitePool,
        input: CreateMcpServerInput,
    ) -> Result<McpServerRecord, AppError> {
        if input.name.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("name 不能为空")));
        }
        Self::validate_transport(&input.transport)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO mcp_server_registry (id, name, transport, command, args_json, env_json, permission_scope, enabled, is_deleted, created_at, display_name, description, health_status, last_health_check, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 1, 0, ?, ?, ?, 'unknown', NULL, ?)",
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&input.transport)
        .bind(&input.command)
        .bind(&input.args_json)
        .bind(&input.env_json)
        .bind(&input.permission_scope)
        .bind(&now)
        .bind(&input.display_name)
        .bind(&input.description)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_mcp_server",
            "mcp_server_registry",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_mcp_server(pool, &id).await
    }

    /// 更新 MCP 服务器。
    pub async fn update_mcp_server(
        pool: &SqlitePool,
        id: &str,
        input: UpdateMcpServerInput,
    ) -> Result<McpServerRecord, AppError> {
        Self::get_mcp_server(pool, id).await?;

        if let Some(enabled) = input.enabled {
            if enabled != 0 && enabled != 1 {
                return Err(AppError::InvalidInput(String::from(
                    "enabled 仅支持 0 或 1",
                )));
            }
        }

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE mcp_server_registry SET display_name = COALESCE(?, display_name), description = COALESCE(?, description), command = COALESCE(?, command), args_json = COALESCE(?, args_json), env_json = COALESCE(?, env_json), permission_scope = COALESCE(?, permission_scope), enabled = COALESCE(?, enabled), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.display_name)
        .bind(&input.description)
        .bind(&input.command)
        .bind(&input.args_json)
        .bind(&input.env_json)
        .bind(&input.permission_scope)
        .bind(input.enabled)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("MCP 服务器不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "update_mcp_server",
            "mcp_server_registry",
            Some(id),
            "medium",
            false,
        )
        .await?;

        Self::get_mcp_server(pool, id).await
    }

    /// 软删除 MCP 服务器。
    pub async fn delete_mcp_server(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE mcp_server_registry SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("MCP 服务器不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_mcp_server",
            "mcp_server_registry",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 检查 MCP 服务器健康状态并写回数据库。
    pub async fn check_mcp_health(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<McpHealthResult, AppError> {
        let server = Self::get_mcp_server(pool, id).await?;
        let checked_at = Utc::now().to_rfc3339();

        let (health_status, message) = if server.transport == "stdio" {
            Self::check_stdio_health(&server).await
        } else if server.transport == "http" {
            (
                String::from("unknown"),
                String::from("HTTP MCP 当前版本默认关闭"),
            )
        } else {
            (
                String::from("unknown"),
                String::from("未知 transport，无法执行健康检查"),
            )
        };

        sqlx::query(
            "UPDATE mcp_server_registry SET health_status = ?, last_health_check = ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&health_status)
        .bind(&checked_at)
        .bind(&checked_at)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(McpHealthResult {
            name: server.name,
            health_status,
            message,
            checked_at,
        })
    }

    /// 校验 transport 是否在允许范围内。
    fn validate_transport(transport: &str) -> Result<(), AppError> {
        match transport {
            "stdio" | "http" => Ok(()),
            _ => Err(AppError::InvalidInput(String::from(
                "transport 仅支持 stdio 或 http",
            ))),
        }
    }

    /// 执行 stdio MCP 的进程级健康检查。
    async fn check_stdio_health(server: &McpServerRecord) -> (String, String) {
        let Some(command) = server.command.as_deref() else {
            return (
                String::from("unhealthy"),
                String::from("stdio 模式缺少 command 配置"),
            );
        };
        if command.trim().is_empty() {
            return (
                String::from("unhealthy"),
                String::from("stdio 模式 command 不能为空"),
            );
        }

        let args = match server.args_json.as_deref() {
            Some(raw) if !raw.trim().is_empty() => match serde_json::from_str::<Vec<String>>(raw) {
                Ok(parsed) => parsed,
                Err(error) => {
                    return (
                        String::from("unhealthy"),
                        format!("args_json 解析失败：{error}"),
                    );
                }
            },
            _ => Vec::new(),
        };

        let envs = match server.env_json.as_deref() {
            Some(raw) if !raw.trim().is_empty() => {
                match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(raw) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        return (
                            String::from("unhealthy"),
                            format!("env_json 解析失败：{error}"),
                        );
                    }
                }
            }
            _ => serde_json::Map::new(),
        };

        let mut cmd = Command::new(command);
        if !args.is_empty() {
            cmd.args(args);
        }
        for (key, value) in envs {
            if let Some(value_str) = value.as_str() {
                cmd.env(key, value_str);
            }
        }

        match cmd.spawn() {
            Ok(mut child) => {
                let _ = child.kill().await;
                (
                    String::from("healthy"),
                    String::from("stdio MCP 进程可正常拉起"),
                )
            }
            Err(error) => (
                String::from("unhealthy"),
                format!("stdio MCP 启动失败：{error}"),
            ),
        }
    }
}

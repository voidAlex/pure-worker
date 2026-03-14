//! MCP 运行时客户端（WP-AI-007）
//!
//! 实现 MCP (Model Context Protocol) 客户端，支持：
//! - 连接 MCP 服务器 (stdio/http)
//! - tools/list - 获取可用工具列表
//! - tools/call - 调用工具
//! - 集成到 Tool Registry

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::error::AppError;
use crate::models::mcp_server::McpServerRecord;

/// MCP JSON-RPC 请求
#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// MCP JSON-RPC 响应
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct McpResponse {
    jsonrpc: String,
    id: u64,
    #[serde(flatten)]
    result: McpResult,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum McpResult {
    Success { result: serde_json::Value },
    Error { error: McpError },
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct McpError {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

/// MCP 工具定义
#[derive(Debug, Clone, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

/// MCP 客户端
#[allow(dead_code)]
pub struct McpClient {
    server: McpServerRecord,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    child: Child,
    request_id: u64,
}

impl McpClient {
    /// 创建并连接 MCP 客户端
    pub async fn connect(server: McpServerRecord) -> Result<Self, AppError> {
        if server.transport != "stdio" {
            return Err(AppError::InvalidInput(format!(
                "不支持的 transport 类型: {}",
                server.transport
            )));
        }

        let command = server
            .command
            .as_deref()
            .ok_or_else(|| AppError::InvalidInput(String::from("stdio 模式需要 command")))?;

        let args: Vec<String> = match server.args_json.as_deref() {
            Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
                .map_err(|e| AppError::Config(format!("args_json 解析失败: {}", e)))?,
            _ => vec![],
        };

        let mut cmd = Command::new(command);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        // 设置环境变量
        if let Some(env_json) = server.env_json.as_deref() {
            if !env_json.trim().is_empty() {
                let envs: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(env_json)
                        .map_err(|e| AppError::Config(format!("env_json 解析失败: {}", e)))?;
                for (key, value) in envs {
                    if let Some(val_str) = value.as_str() {
                        cmd.env(key, val_str);
                    }
                }
            }
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| AppError::ExternalService(format!("启动 MCP 服务器失败: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Internal(String::from("无法获取 stdin")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Internal(String::from("无法获取 stdout")))?;

        let mut client = Self {
            server,
            stdin,
            stdout: BufReader::new(stdout),
            child,
            request_id: 0,
        };

        // 初始化连接
        client.initialize().await?;

        Ok(client)
    }

    /// 发送初始化请求
    async fn initialize(&mut self) -> Result<(), AppError> {
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "pure-worker",
                    "version": "0.1.0"
                }
            }
        });

        self.send_raw(&init_request.to_string()).await?;
        let _ = self.receive_raw().await?; // 接收初始化响应

        // 发送 initialized 通知
        let initialized = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        self.send_raw(&initialized.to_string()).await?;

        Ok(())
    }

    /// 获取工具列表
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>, AppError> {
        let request_id = self.next_id();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "tools/list".to_string(),
            params: json!({}),
        };

        self.send(request).await?;
        let response = self.receive().await?;

        match response.result {
            McpResult::Success { result } => {
                let tools: Vec<McpTool> = serde_json::from_value(
                    result.get("tools").cloned().unwrap_or_else(|| json!([])),
                )
                .map_err(|e| AppError::ExternalService(format!("解析工具列表失败: {}", e)))?;
                Ok(tools)
            }
            McpResult::Error { error } => Err(AppError::ExternalService(format!(
                "MCP 错误 ({}): {}",
                error.code, error.message
            ))),
        }
    }

    /// 调用工具
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let request_id = self.next_id();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "tools/call".to_string(),
            params: json!({
                "name": tool_name,
                "arguments": arguments
            }),
        };

        self.send(request).await?;
        let response = self.receive().await?;

        match response.result {
            McpResult::Success { result } => Ok(result),
            McpResult::Error { error } => Err(AppError::ExternalService(format!(
                "MCP 工具调用错误 ({}): {}",
                error.code, error.message
            ))),
        }
    }

    /// 发送请求
    async fn send(&mut self, request: McpRequest) -> Result<(), AppError> {
        let json = serde_json::to_string(&request)
            .map_err(|e| AppError::Internal(format!("序列化请求失败: {}", e)))?;
        self.send_raw(&json).await
    }

    /// 发送原始 JSON
    async fn send_raw(&mut self, json: &str) -> Result<(), AppError> {
        let message = format!("{}\n", json);
        self.stdin
            .write_all(message.as_bytes())
            .await
            .map_err(|e| AppError::ExternalService(format!("发送 MCP 请求失败: {}", e)))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| AppError::ExternalService(format!("刷新 MCP 流失败: {}", e)))?;
        Ok(())
    }

    /// 接收响应
    async fn receive(&mut self) -> Result<McpResponse, AppError> {
        let line = self.receive_raw().await?;
        let response: McpResponse = serde_json::from_str(&line)
            .map_err(|e| AppError::ExternalService(format!("解析 MCP 响应失败: {}", e)))?;
        Ok(response)
    }

    /// 接收原始行
    async fn receive_raw(&mut self) -> Result<String, AppError> {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .await
            .map_err(|e| AppError::ExternalService(format!("读取 MCP 响应失败: {}", e)))?;
        Ok(line)
    }

    /// 获取下一个请求 ID
    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// MCP 运行时管理器
pub struct McpRuntime;

impl McpRuntime {
    /// 从数据库加载所有启用的 MCP 服务器并获取它们的工具
    pub async fn load_all_tools(
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<(McpServerRecord, Vec<McpTool>)>, AppError> {
        let servers = sqlx::query_as::<_, McpServerRecord>(
            "SELECT id, name, transport, command, args_json, env_json, permission_scope, enabled, is_deleted, created_at, display_name, description, health_status, last_health_check, updated_at FROM mcp_server_registry WHERE enabled = 1 AND is_deleted = 0 AND health_status = 'healthy'",
        )
        .fetch_all(pool)
        .await?;

        let mut results = Vec::new();
        for server in servers {
            match McpClient::connect(server.clone()).await {
                Ok(mut client) => match client.list_tools().await {
                    Ok(tools) => {
                        results.push((server, tools));
                    }
                    Err(e) => {
                        eprintln!("获取 MCP 服务器 '{}' 工具列表失败: {}", server.name, e);
                    }
                },
                Err(e) => {
                    eprintln!("连接 MCP 服务器 '{}' 失败: {}", server.name, e);
                }
            }
        }

        Ok(results)
    }

    /// 调用指定服务器的工具
    pub async fn invoke_tool(
        pool: &sqlx::SqlitePool,
        server_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let server = sqlx::query_as::<_, McpServerRecord>(
            "SELECT id, name, transport, command, args_json, env_json, permission_scope, enabled, is_deleted, created_at, display_name, description, health_status, last_health_check, updated_at FROM mcp_server_registry WHERE id = ? AND enabled = 1 AND is_deleted = 0",
        )
        .bind(server_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("MCP 服务器不存在或未启用: {}", server_id)))?;

        let mut client = McpClient::connect(server).await?;
        client.call_tool(tool_name, arguments).await
    }
}

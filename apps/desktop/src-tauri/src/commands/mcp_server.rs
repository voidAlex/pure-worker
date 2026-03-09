//! MCP 服务器注册 IPC 命令模块
//!
//! 暴露 MCP 服务器注册增删改查与健康检查能力给前端调用。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::mcp_server::{
    CreateMcpServerInput, McpHealthResult, McpServerRecord, UpdateMcpServerInput,
};
use crate::services::mcp_server::McpServerService;

/// 删除 MCP 服务器输入。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteMcpServerInput {
    pub id: String,
}

/// 删除 MCP 服务器响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteMcpServerResponse {
    pub success: bool,
}

/// 列出 MCP 服务器。
#[tauri::command]
#[specta::specta]
pub async fn list_mcp_servers(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<McpServerRecord>, AppError> {
    McpServerService::list_mcp_servers(&pool).await
}

/// 获取单个 MCP 服务器。
#[tauri::command]
#[specta::specta]
pub async fn get_mcp_server(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<McpServerRecord, AppError> {
    McpServerService::get_mcp_server(&pool, &id).await
}

/// 创建 MCP 服务器。
#[tauri::command]
#[specta::specta]
pub async fn create_mcp_server(
    pool: State<'_, SqlitePool>,
    input: CreateMcpServerInput,
) -> Result<McpServerRecord, AppError> {
    McpServerService::create_mcp_server(&pool, input).await
}

/// 更新 MCP 服务器。
#[tauri::command]
#[specta::specta]
pub async fn update_mcp_server(
    pool: State<'_, SqlitePool>,
    id: String,
    input: UpdateMcpServerInput,
) -> Result<McpServerRecord, AppError> {
    McpServerService::update_mcp_server(&pool, &id, input).await
}

/// 删除 MCP 服务器。
#[tauri::command]
#[specta::specta]
pub async fn delete_mcp_server(
    pool: State<'_, SqlitePool>,
    input: DeleteMcpServerInput,
) -> Result<DeleteMcpServerResponse, AppError> {
    McpServerService::delete_mcp_server(&pool, &input.id).await?;
    Ok(DeleteMcpServerResponse { success: true })
}

/// 检查 MCP 服务器健康状态。
#[tauri::command]
#[specta::specta]
pub async fn check_mcp_health(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<McpHealthResult, AppError> {
    McpServerService::check_mcp_health(&pool, &id).await
}

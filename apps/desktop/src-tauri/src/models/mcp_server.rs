//! MCP 服务注册数据模型模块
//!
//! 定义 MCP 服务注册记录、创建/更新输入以及健康检查结果结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// MCP 服务注册记录。
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct McpServerRecord {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args_json: Option<String>,
    pub env_json: Option<String>,
    pub permission_scope: Option<String>,
    pub enabled: i32,
    pub is_deleted: i32,
    pub created_at: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub health_status: String,
    pub last_health_check: Option<String>,
    pub updated_at: Option<String>,
}

/// 创建 MCP 服务输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateMcpServerInput {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args_json: Option<String>,
    pub env_json: Option<String>,
    pub permission_scope: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

/// 更新 MCP 服务输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdateMcpServerInput {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub command: Option<String>,
    pub args_json: Option<String>,
    pub env_json: Option<String>,
    pub permission_scope: Option<String>,
    pub enabled: Option<i32>,
}

/// MCP 服务健康检查结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct McpHealthResult {
    pub name: String,
    pub health_status: String,
    pub message: String,
    pub checked_at: String,
}

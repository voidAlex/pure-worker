//! 对话会话数据模型
//!
//! 定义会话(conversation)和消息(conversation_message)的数据结构

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::FromRow;

/// 会话实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct Conversation {
    pub id: String,
    pub teacher_id: String,
    pub title: Option<String>,
    pub scenario: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 会话消息实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct ConversationMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
}

/// 创建会话输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateConversationInput {
    pub teacher_id: String,
    pub title: Option<String>,
    pub scenario: Option<String>,
}

/// 更新会话输入
#[derive(Debug, Deserialize, Type)]
pub struct UpdateConversationInput {
    pub id: String,
    pub title: Option<String>,
    pub scenario: Option<String>,
}

/// 创建消息输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateMessageInput {
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
}

/// 会话列表项（简化输出）
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct ConversationListItem {
    pub id: String,
    pub title: Option<String>,
    pub scenario: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
}

/// 消息列表项
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct MessageListItem {
    pub id: String,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub created_at: String,
}

/// 流式聊天请求输入
#[derive(Debug, Deserialize, Type)]
pub struct ChatStreamInput {
    pub conversation_id: Option<String>,
    pub message: String,
    pub agent_role: String,
    /// 是否启用 Agentic Search 自动检索上下文
    pub use_agentic_search: Option<bool>,
}

/// 流式聊天事件类型
#[derive(Debug, Clone, Serialize, Type)]
pub enum ChatStreamEvent {
    #[serde(rename = "Start")]
    Start { message_id: String },
    #[serde(rename = "Chunk")]
    Chunk { content: String },
    #[serde(rename = "Complete")]
    Complete,
    #[serde(rename = "Error")]
    Error { message: String },
}

//! 对话会话 IPC 命令模块
//!
//! 提供前端会话管理的 IPC 接口

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::conversation::{
    Conversation, ConversationListItem, CreateConversationInput, MessageListItem,
    UpdateConversationInput,
};
use crate::services::conversation_service::ConversationService;

/// 列出会话请求
#[derive(Debug, Deserialize, Type)]
pub struct ListConversationsInput {
    pub teacher_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 列出会话响应
#[derive(Debug, Serialize, Type)]
pub struct ListConversationsResponse {
    pub conversations: Vec<ConversationListItem>,
    pub total: i64,
}

/// 获取消息请求
#[derive(Debug, Deserialize, Type)]
pub struct GetMessagesInput {
    pub conversation_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 创建会话 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn create_conversation(
    pool: State<'_, SqlitePool>,
    input: CreateConversationInput,
) -> Result<Conversation, AppError> {
    ConversationService::create_conversation(&pool, input).await
}

/// 列出会话 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn list_conversations(
    pool: State<'_, SqlitePool>,
    input: ListConversationsInput,
) -> Result<ListConversationsResponse, AppError> {
    let limit = input.limit.unwrap_or(50);
    let offset = input.offset.unwrap_or(0);

    let conversations =
        ConversationService::list_conversations(&pool, &input.teacher_id, limit, offset).await?;
    let total = conversations.len() as i64;

    Ok(ListConversationsResponse {
        conversations,
        total,
    })
}

/// 获取会话详情 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn get_conversation(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<Conversation, AppError> {
    ConversationService::get_conversation_by_id(&pool, &id).await
}

/// 更新会话 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn update_conversation(
    pool: State<'_, SqlitePool>,
    input: UpdateConversationInput,
) -> Result<Conversation, AppError> {
    ConversationService::update_conversation(&pool, input).await
}

/// 删除会话 IPC 命令（软删除）
#[tauri::command]
#[specta::specta]
pub async fn delete_conversation(pool: State<'_, SqlitePool>, id: String) -> Result<(), AppError> {
    ConversationService::delete_conversation(&pool, &id).await
}

/// 获取会话消息列表 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn list_conversation_messages(
    pool: State<'_, SqlitePool>,
    input: GetMessagesInput,
) -> Result<Vec<MessageListItem>, AppError> {
    let limit = input.limit.unwrap_or(100);
    let offset = input.offset.unwrap_or(0);

    ConversationService::list_messages(&pool, &input.conversation_id, limit, offset).await
}

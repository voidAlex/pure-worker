//! AI 通用聊天 IPC 命令模块（兼容性包装）
//!
//! 本模块已转为兼容性包装层，核心逻辑已迁移到 execution 模块。
//! 保留这些命令以保持向后兼容性，但内部实现委托给新的执行编排器。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::conversation::{ChatStreamInput, ConversationListItem};
use crate::services::conversation_service::ConversationService;

/// 聊天请求输入（旧版格式）
#[derive(Debug, Deserialize, Type)]
pub struct ChatInput {
    /// 用户消息内容
    pub message: String,
    /// AI 角色标识（homeroom/grading/communication/ops）
    pub agent_role: String,
    /// 是否启用 Agentic Search 自动检索上下文
    pub use_agentic_search: Option<bool>,
}

/// 聊天响应输出
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChatResponse {
    /// AI 回复内容
    pub content: String,
    /// 使用的模型名称
    pub model: String,
}

/// 与 AI 进行通用对话（非流式）
///
/// 兼容性包装：委托给新的执行编排器
#[tauri::command]
#[specta::specta]
pub async fn chat_with_ai(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: ChatInput,
) -> Result<ChatResponse, AppError> {
    if input.message.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("消息内容不能为空")));
    }

    // 转换为新的 ChatStreamInput 并调用新的执行流程
    let stream_input = ChatStreamInput {
        conversation_id: None,
        message: input.message,
        agent_role: input.agent_role.clone(),
        use_agentic_search: input.use_agentic_search,
    };

    // 委托给新的执行命令
    let _session_id =
        crate::commands::execution::execute_from_chat_input(app_handle, pool, stream_input).await?;

    // 由于新的执行流程是流式的，这里返回一个兼容性响应
    // 实际内容会通过事件发送给前端
    Ok(ChatResponse {
        content: String::from("[流式响应已启动，请监听 chat-stream 事件]"),
        model: String::from("delegated"),
    })
}

/// 流式 AI 对话命令（兼容性包装）
///
/// 委托给新的执行编排器的 execute_stream 命令
#[tauri::command]
#[specta::specta]
pub async fn chat_stream(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: ChatStreamInput,
) -> Result<String, AppError> {
    // 委托给新的执行命令，保持事件通道为 "chat-stream" 以兼容前端
    crate::commands::execution::execute_from_chat_input(app, pool, input).await
}

/// 获取对话列表
#[tauri::command]
#[specta::specta]
pub async fn list_chat_conversations(
    pool: State<'_, SqlitePool>,
    page: i64,
    page_size: i64,
) -> Result<Vec<ConversationListItem>, AppError> {
    let teacher_id = get_current_teacher_id(&pool).await?;
    let offset = page * page_size;
    ConversationService::list_conversations(&pool, &teacher_id, page_size, offset).await
}

/// 获取对话详情（包含消息列表）
#[tauri::command]
#[specta::specta]
pub async fn get_chat_conversation(
    pool: State<'_, SqlitePool>,
    conversation_id: String,
) -> Result<Vec<crate::models::conversation::MessageListItem>, AppError> {
    ConversationService::list_messages(&pool, &conversation_id, 100, 0).await
}

/// 删除对话
#[tauri::command]
#[specta::specta]
pub async fn delete_chat_conversation(
    pool: State<'_, SqlitePool>,
    conversation_id: String,
) -> Result<(), AppError> {
    ConversationService::delete_conversation(&pool, &conversation_id).await
}

/// 获取当前教师ID
async fn get_current_teacher_id(pool: &SqlitePool) -> Result<String, AppError> {
    let teacher_id: Option<String> =
        sqlx::query_scalar("SELECT id FROM teacher_profile WHERE is_deleted = 0 LIMIT 1")
            .fetch_optional(pool)
            .await?;

    teacher_id.ok_or_else(|| {
        AppError::NotFound(String::from(
            "请先完成教师信息初始化。如果您已完成初始化向导，请尝试重启应用。",
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证兼容性命令结构
    #[test]
    fn test_chat_input_structure() {
        let input = ChatInput {
            message: "测试消息".to_string(),
            agent_role: "homeroom".to_string(),
            use_agentic_search: Some(true),
        };

        assert_eq!(input.message, "测试消息");
        assert_eq!(input.agent_role, "homeroom");
        assert!(input.use_agentic_search.unwrap());
    }
}

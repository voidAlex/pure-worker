//! AI 通用聊天 IPC 命令模块
//!
//! 提供前端 AI 助手面板的通用对话能力。

use rig::completion::Prompt;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::Emitter;
use tauri::State;

use crate::error::AppError;
use crate::models::conversation::{
    ChatStreamEvent, ChatStreamInput, CreateConversationInput, CreateMessageInput,
};
use crate::services::conversation_service::ConversationService;
use crate::services::llm_provider::LlmProviderService;
use crate::services::skill_tool_adapter::build_all_enabled_skill_tools;

/// 聊天请求输入。
#[derive(Debug, Deserialize, Type)]
pub struct ChatInput {
    /// 用户消息内容。
    pub message: String,
    /// AI 角色标识（homeroom/grading/communication/ops）。
    pub agent_role: String,
}

/// 聊天响应输出。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChatResponse {
    /// AI 回复内容。
    pub content: String,
    /// 使用的模型名称。
    pub model: String,
}

/// 获取角色对应的系统提示词。
fn get_system_prompt(agent_role: &str) -> &'static str {
    match agent_role {
        "homeroom" => "你是一名经验丰富的班主任助手。你帮助教师处理班级管理、学生行为记录、家校沟通等日常工作。回答简洁实用，符合中国中小学教育场景。",
        "grading" => "你是一名专业的批改助手。你帮助教师批改作业、分析成绩、生成评语和练习题。回答专业准确，关注学生学习进步。",
        "communication" => "你是一名家校沟通助手。你帮助教师撰写家长通知、沟通话术、活动公告等文案。语言温暖得体，兼顾专业性与亲和力。",
        "ops" => "你是一名教务助手。你帮助教师处理课表安排、教学计划、行政事务等工作。回答条理清晰，注重效率。",
        _ => "你是 PureWorker 教务 AI 助手，帮助教师高效完成日常教务工作。回答简洁实用，符合中国中小学教育场景。",
    }
}

/// 与 AI 进行通用对话。
#[tauri::command]
#[specta::specta]
pub async fn chat_with_ai(
    pool: State<'_, SqlitePool>,
    input: ChatInput,
) -> Result<ChatResponse, AppError> {
    if input.message.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("消息内容不能为空")));
    }

    let config = LlmProviderService::get_active_config(&pool).await?;
    let model_name = config.default_model.clone();
    let client = LlmProviderService::create_client(&config)?;
    let system_prompt = get_system_prompt(&input.agent_role);

    let skill_tools = build_all_enabled_skill_tools(&pool)
        .await
        .unwrap_or_default();

    let response: String = if skill_tools.is_empty() {
        let agent =
            LlmProviderService::create_agent(&client, &config.default_model, system_prompt, 0.7);
        agent
            .prompt(&input.message)
            .await
            .map_err(|error| AppError::ExternalService(format!("AI 对话调用失败：{error}")))?
    } else {
        let agent = LlmProviderService::create_agent_with_tools(
            &client,
            &config.default_model,
            system_prompt,
            0.7,
            skill_tools,
        );
        agent
            .prompt(&input.message)
            .await
            .map_err(|error| AppError::ExternalService(format!("AI 对话调用失败：{error}")))?
    };

    Ok(ChatResponse {
        content: response,
        model: model_name,
    })
}

/// 流式 AI 对话命令
#[tauri::command]
#[specta::specta]
pub async fn chat_stream(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: ChatStreamInput,
) -> Result<String, AppError> {
    if input.message.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("消息内容不能为空")));
    }

    // 获取或创建会话
    let conversation_id = if let Some(id) = input.conversation_id {
        ConversationService::get_conversation_by_id(&pool, &id).await?;
        id
    } else {
        let teacher_id = get_current_teacher_id(&pool).await?;
        let title = ConversationService::generate_title(&input.message);
        let conversation = ConversationService::create_conversation(
            &pool,
            CreateConversationInput {
                teacher_id,
                title: Some(title),
                scenario: Some(input.agent_role.clone()),
            },
        )
        .await?;
        conversation.id
    };

    // 保存用户消息
    let _user_message = ConversationService::create_message(
        &pool,
        CreateMessageInput {
            conversation_id: conversation_id.clone(),
            role: "user".to_string(),
            content: input.message.clone(),
            tool_name: None,
        },
    )
    .await?;

    // 创建 AI 消息占位
    let assistant_message = ConversationService::create_message(
        &pool,
        CreateMessageInput {
            conversation_id: conversation_id.clone(),
            role: "assistant".to_string(),
            content: String::new(),
            tool_name: None,
        },
    )
    .await?;

    // 发送开始事件
    let _ = app.emit(
        "chat-stream",
        ChatStreamEvent::Start {
            message_id: assistant_message.id.clone(),
        },
    );

    // 获取 AI 配置并生成响应（简化版 - 非流式）
    let config = LlmProviderService::get_active_config(&pool).await?;
    let _client = LlmProviderService::create_client(&config)?;

    // 获取对话历史
    let _history =
        ConversationService::get_conversation_history(&pool, &conversation_id, 20).await?;

    // TODO: 实现真正的流式生成
    // 目前先使用简单实现，逐字发送模拟流式效果
    let full_content = "这是一条模拟的 AI 响应消息。实际实现需要接入 Rig 的流式 API。".to_string();

    // 模拟流式发送
    for chunk in full_content.chars().collect::<Vec<_>>().chunks(5) {
        let chunk_str: String = chunk.iter().collect();
        let _ = app.emit("chat-stream", ChatStreamEvent::Chunk { content: chunk_str });
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // 更新 AI 消息
    let _ =
        ConversationService::update_message_content(&pool, &assistant_message.id, &full_content)
            .await;

    // 发送完成事件
    let _ = app.emit("chat-stream", ChatStreamEvent::Complete);

    Ok(conversation_id)
}

/// 获取当前教师ID
async fn get_current_teacher_id(pool: &SqlitePool) -> Result<String, AppError> {
    let teacher_id: Option<String> =
        sqlx::query_scalar("SELECT id FROM teacher_profile WHERE is_deleted = 0 LIMIT 1")
            .fetch_optional(pool)
            .await?;

    teacher_id.ok_or_else(|| AppError::NotFound(String::from("未找到教师档案")))
}

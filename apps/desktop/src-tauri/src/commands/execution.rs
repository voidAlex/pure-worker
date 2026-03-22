//! 执行运行时 IPC 命令模块
//!
//! 提供统一的 AI 执行运行时命令，支持流式和非流式执行模式。
//! 作为新的主链入口，替代旧的 chat 命令的核心逻辑。

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::Emitter;
use tauri::State;

use crate::error::AppError;
use crate::models::execution::{ExecutionRequest, SessionEvent};
use crate::services::ai_orchestration::agent_profile_registry::AgentProfileRegistry;
use crate::services::ai_orchestration::execution_request_factory::ExecutionRequestFactory;
use crate::services::ai_orchestration::session_event_bus::SessionEventBus;
use crate::services::ai_orchestration::ExecutionOrchestratorBuilder;
use crate::services::runtime_paths;
use crate::services::tool_registry::get_registry;

/// 流式执行请求输入
#[derive(Debug, Deserialize, Type)]
pub struct StreamExecutionInput {
    /// 执行请求
    pub request: ExecutionRequest,
    /// 前端事件通道名称（默认为 "execution-stream"）
    pub event_channel: Option<String>,
}

/// 非流式执行请求输入
#[derive(Debug, Deserialize, Type)]
pub struct ExecuteInput {
    /// 执行请求
    pub request: ExecutionRequest,
}

/// 执行结果输出
#[derive(Debug, Clone, Serialize, Type)]
pub struct ExecutionResult {
    /// 生成的内容
    pub content: String,
    /// 使用的模型ID
    pub model_id: String,
    /// 会话ID
    pub session_id: String,
    /// 消息ID
    pub message_id: String,
}

/// 流式执行响应
#[derive(Debug, Clone, Serialize, Type)]
pub enum StreamExecutionEvent {
    /// 执行开始
    #[serde(rename = "Start")]
    Start { message_id: String },
    /// 思考状态更新
    #[serde(rename = "ThinkingStatus")]
    ThinkingStatus { stage: String, description: String },
    /// 搜索摘要
    #[serde(rename = "SearchSummary")]
    SearchSummary {
        sources: Vec<String>,
        evidence_count: usize,
    },
    /// 推理摘要
    #[serde(rename = "Reasoning")]
    Reasoning { summary: String },
    /// 执行摘要
    #[serde(rename = "ExecutionSummary")]
    ExecutionSummary { status: String, used_model: String },
    /// 内容片段
    #[serde(rename = "Chunk")]
    Chunk { content: String },
    /// 执行完成
    #[serde(rename = "Complete")]
    Complete,
    /// 执行错误
    #[serde(rename = "Error")]
    Error { message: String },
}

/// 执行非流式 AI 请求
#[tauri::command]
#[specta::specta]
pub async fn execute(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: ExecuteInput,
) -> Result<ExecutionResult, AppError> {
    if input.request.user_input.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("输入内容不能为空")));
    }

    // 获取工作区路径
    let workspace_path = runtime_paths::resolve_workspace_path(&app)?;

    // 创建事件总线
    let event_bus = SessionEventBus::new();

    // 创建 Profile 注册表
    let profile_registry = AgentProfileRegistry::new_default();

    // 获取工具注册表
    let tool_registry = get_registry();

    // 构建编排器
    let orchestrator = ExecutionOrchestratorBuilder::new(&pool)
        .with_profile_registry(&profile_registry)
        .with_event_bus(&event_bus)
        .with_tool_registry(tool_registry)
        .with_workspace_path(workspace_path)
        .build()?;

    // 生成会话ID和消息ID
    let session_id = input
        .request
        .session_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let message_id = uuid::Uuid::new_v4().to_string();

    // 执行请求
    let artifacts = orchestrator.execute(&input.request).await?;

    Ok(ExecutionResult {
        content: artifacts.content,
        model_id: artifacts.model_id,
        session_id,
        message_id,
    })
}

/// 执行流式 AI 请求
#[tauri::command]
#[specta::specta]
pub async fn execute_stream(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: StreamExecutionInput,
) -> Result<String, AppError> {
    if input.request.user_input.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("输入内容不能为空")));
    }

    let event_channel = input
        .event_channel
        .unwrap_or_else(|| "execution-stream".to_string());

    // 获取工作区路径
    let workspace_path = runtime_paths::resolve_workspace_path(&app)?;

    // 创建事件总线
    let event_bus = SessionEventBus::new();

    // 创建 Profile 注册表
    let profile_registry = AgentProfileRegistry::new_default();

    // 获取工具注册表
    let tool_registry = get_registry();

    // 构建编排器
    let orchestrator = ExecutionOrchestratorBuilder::new(&pool)
        .with_profile_registry(&profile_registry)
        .with_event_bus(&event_bus)
        .with_tool_registry(tool_registry)
        .with_workspace_path(workspace_path)
        .build()?;

    // 生成会话ID
    let session_id = input
        .request
        .session_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // 发送开始事件
    let message_id = uuid::Uuid::new_v4().to_string();
    let _ = app.emit(
        &event_channel,
        StreamExecutionEvent::Start {
            message_id: message_id.clone(),
        },
    );

    // 执行流式请求
    let mut stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<SessionEvent, AppError>> + Send>,
    > = orchestrator.execute_streaming(&input.request).await?;

    // 消费流并转发事件到前端
    let mut accumulated_content = String::new();

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(SessionEvent::Chunk { content, .. }) => {
                accumulated_content.push_str(&content);
                let _ = app.emit(&event_channel, StreamExecutionEvent::Chunk { content });
            }
            Ok(SessionEvent::Complete { .. }) => {
                let _ = app.emit(&event_channel, StreamExecutionEvent::Complete);
            }
            Ok(SessionEvent::Error { message, .. }) => {
                let _ = app.emit(
                    &event_channel,
                    StreamExecutionEvent::Error {
                        message: message.clone(),
                    },
                );
                return Err(AppError::TaskExecution(message));
            }
            Ok(SessionEvent::SearchSummary {
                sources,
                evidence_count,
                ..
            }) => {
                let _ = app.emit(
                    &event_channel,
                    StreamExecutionEvent::SearchSummary {
                        sources,
                        evidence_count,
                    },
                );
            }
            Ok(SessionEvent::Reasoning { summary, .. }) => {
                let _ = app.emit(&event_channel, StreamExecutionEvent::Reasoning { summary });
            }
            Ok(SessionEvent::ThinkingStatus {
                stage, description, ..
            }) => {
                let _ = app.emit(
                    &event_channel,
                    StreamExecutionEvent::ThinkingStatus { stage, description },
                );
            }
            Ok(SessionEvent::Start { message_id, .. }) => {
                // 开始事件已在上面发送
                let _ = message_id;
            }
            Ok(SessionEvent::ExecutionSummary {
                status, used_model, ..
            }) => {
                // 转发执行摘要事件到前端
                let _ = app.emit(
                    &event_channel,
                    StreamExecutionEvent::ExecutionSummary { status, used_model },
                );
            }
            Err(e) => {
                let error_msg = e.to_string();
                let _ = app.emit(
                    &event_channel,
                    StreamExecutionEvent::Error {
                        message: error_msg.clone(),
                    },
                );
                return Err(e);
            }
            _ => {}
        }
    }

    Ok(session_id)
}

/// 从旧的 ChatStreamInput 执行流式请求（兼容性包装）
#[tauri::command]
#[specta::specta]
pub async fn execute_from_chat_input(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    chat_input: crate::models::conversation::ChatStreamInput,
) -> Result<String, AppError> {
    use crate::services::conversation_service::ConversationService;

    // 获取或创建会话
    let conversation_id = if let Some(id) = chat_input.conversation_id.clone() {
        ConversationService::get_conversation_by_id(&pool, &id).await?;
        id
    } else {
        let teacher_id = get_current_teacher_id(&pool).await?;
        let title = ConversationService::generate_title(&chat_input.message);
        let conversation = ConversationService::create_conversation(
            &pool,
            crate::models::conversation::CreateConversationInput {
                teacher_id,
                title: Some(title),
                scenario: Some(chat_input.agent_role.clone()),
            },
        )
        .await?;
        conversation.id
    };

    // 保存用户消息
    let _user_message = ConversationService::create_message(
        &pool,
        crate::models::conversation::CreateMessageInput {
            conversation_id: conversation_id.clone(),
            role: "user".to_string(),
            content: chat_input.message.clone(),
            tool_name: None,
        },
    )
    .await?;

    // 创建 AI 消息占位
    let _assistant_message = ConversationService::create_message(
        &pool,
        crate::models::conversation::CreateMessageInput {
            conversation_id: conversation_id.clone(),
            role: "assistant".to_string(),
            content: String::new(),
            tool_name: None,
        },
    )
    .await?;

    // 转换为 ExecutionRequest
    let request =
        ExecutionRequestFactory::from_chat_stream_input(&chat_input, Some(conversation_id.clone()));

    // 调用新的流式执行
    execute_stream(
        app,
        pool,
        StreamExecutionInput {
            request,
            event_channel: Some("chat-stream".to_string()),
        },
    )
    .await?;

    Ok(conversation_id)
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
    use crate::models::execution::{ExecutionEntrypoint, ExecutionRequest, StreamMode};

    /// 验证 ExecutionResult 构造
    #[test]
    fn test_execution_result_construction() {
        let result = ExecutionResult {
            content: "测试内容".to_string(),
            model_id: "gpt-4o".to_string(),
            session_id: "session-1".to_string(),
            message_id: "msg-1".to_string(),
        };

        assert_eq!(result.content, "测试内容");
        assert_eq!(result.model_id, "gpt-4o");
    }

    /// 验证 StreamExecutionEvent 序列化
    #[test]
    fn test_stream_event_serialization() {
        let event = StreamExecutionEvent::Start {
            message_id: "msg-123".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Start"));
        assert!(json.contains("msg-123"));
    }

    /// 验证执行请求验证
    #[test]
    fn test_execution_request_validation() {
        let request = ExecutionRequest {
            session_id: None,
            entrypoint: ExecutionEntrypoint::Chat,
            agent_profile_id: "chat.homeroom".to_string(),
            user_input: "   ".to_string(), // 空白输入
            attachments: vec![],
            use_agentic_search: false,
            stream_mode: StreamMode::Streaming,
            metadata_json: None,
        };

        assert!(request.user_input.trim().is_empty());
    }
}

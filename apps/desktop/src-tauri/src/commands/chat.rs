//! AI 通用聊天 IPC 命令模块
//!
//! 提供前端 AI 助手面板的通用对话能力。

use rig::completion::Prompt;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::Emitter;
use tauri::Manager;
use tauri::State;

use crate::error::AppError;
use crate::models::agentic_search::AgenticSearchInput;
use crate::models::conversation::{
    ChatStreamEvent, ChatStreamInput, ConversationListItem, CreateConversationInput,
    CreateMessageInput,
};
use crate::services::agentic_search::AgenticSearchOrchestrator;
use crate::services::agentic_search_agent::format_search_result_for_prompt;
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
    /// 是否启用 Agentic Search 自动检索上下文。
    pub use_agentic_search: Option<bool>,
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
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: ChatInput,
) -> Result<ChatResponse, AppError> {
    if input.message.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("消息内容不能为空")));
    }

    let config = LlmProviderService::get_active_config(&pool).await?;
    let model_name = config.default_model.clone();
    let client = LlmProviderService::create_client(&config)?;
    let base_system_prompt = get_system_prompt(&input.agent_role);

    // 如果启用 Agentic Search，执行检索并增强上下文
    let (enhanced_prompt, search_context) = if input.use_agentic_search.unwrap_or(false) {
        let workspace_path = resolve_workspace_path(&app_handle)?;
        let orchestrator = AgenticSearchOrchestrator::new();
        let search_result = orchestrator
            .search(
                &pool,
                &workspace_path,
                AgenticSearchInput {
                    query: input.message.clone(),
                    session_id: None,
                    force_refresh: None,
                },
            )
            .await?;

        let evidence_context = format_search_result_for_prompt(&search_result);
        let enhanced = format!(
            "{}\n\n在回答前，请参考以下检索到的相关证据（如没有相关证据则直接回答）：\n\n{}",
            base_system_prompt, evidence_context
        );

        (enhanced, Some(search_result))
    } else {
        (base_system_prompt.to_string(), None)
    };

    let skill_tools = build_all_enabled_skill_tools(&pool)
        .await
        .unwrap_or_default();

    let response: String = if skill_tools.is_empty() {
        let agent =
            LlmProviderService::create_agent(&client, &config.default_model, &enhanced_prompt, 0.7);
        agent
            .prompt(&input.message)
            .await
            .map_err(|error| AppError::ExternalService(format!("AI 对话调用失败：{error}")))?
    } else {
        let agent = LlmProviderService::create_agent_with_tools(
            &client,
            &config.default_model,
            &enhanced_prompt,
            0.7,
            skill_tools,
        );
        agent
            .prompt(&input.message)
            .await
            .map_err(|error| AppError::ExternalService(format!("AI 对话调用失败：{error}")))?
    };

    // 如果有搜索结果，在响应中附加引用信息
    let final_content = if let Some(search_result) = search_context {
        if !search_result.evidence_sources.is_empty() {
            format!(
                "{}\n\n---\n参考来源：{}",
                response,
                search_result
                    .evidence_sources
                    .iter()
                    .map(|s| format!("[{}]", s.source_type.description()))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        } else {
            response
        }
    } else {
        response
    };

    Ok(ChatResponse {
        content: final_content,
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
    let conversation_id = if let Some(id) = input.conversation_id.clone() {
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

    // 执行流式生成
    let result = stream_chat_response(
        app.clone(),
        &pool,
        &input,
        &assistant_message.id,
        &conversation_id,
    )
    .await;

    // 根据执行结果发送完成或错误事件
    match result {
        Ok(_) => {
            let _ = app.emit("chat-stream", ChatStreamEvent::Complete);
        }
        Err(e) => {
            let error_msg = e.to_string();
            let _ = app.emit(
                "chat-stream",
                ChatStreamEvent::Error {
                    message: error_msg.clone(),
                },
            );
            return Err(e);
        }
    }

    Ok(conversation_id)
}

/// 流式生成聊天响应
///
/// 使用项目适配器的流式 API 逐步生成响应，同时发送事件到前端。
/// 如果生成过程中发生错误，已生成的内容会被保存到数据库。
async fn stream_chat_response(
    app: tauri::AppHandle,
    pool: &SqlitePool,
    input: &ChatStreamInput,
    assistant_message_id: &str,
    conversation_id: &str,
) -> Result<(), AppError> {
    // 获取 AI 配置
    let config = LlmProviderService::get_active_config(pool).await?;
    let base_system_prompt = get_system_prompt(&input.agent_role);

    // 获取对话历史用于上下文
    let _history = ConversationService::get_conversation_history(pool, conversation_id, 20).await?;

    // 如果启用 Agentic Search，执行检索并增强上下文
    let (enhanced_prompt, search_context) = if input.use_agentic_search.unwrap_or(false) {
        // 发送搜索开始状态
        let _ = app.emit(
            "chat-stream",
            ChatStreamEvent::ThinkingStatus {
                stage: String::from("searching"),
                description: String::from("正在检索相关证据..."),
            },
        );

        let workspace_path = resolve_workspace_path(&app)?;
        let orchestrator = AgenticSearchOrchestrator::new();
        let search_result = orchestrator
            .search(
                pool,
                &workspace_path,
                AgenticSearchInput {
                    query: input.message.clone(),
                    session_id: Some(conversation_id.to_string()),
                    force_refresh: None,
                },
            )
            .await;

        match search_result {
            Ok(result) => {
                // 发送搜索结果摘要事件
                let sources: Vec<String> = result
                    .evidence_sources
                    .iter()
                    .map(|s| s.source_type.description().to_string())
                    .collect();
                let evidence_count = result.evidence_sources.len();

                if evidence_count > 0 {
                    let _ = app.emit(
                        "chat-stream",
                        ChatStreamEvent::SearchSummary {
                            sources: sources.clone(),
                            evidence_count,
                        },
                    );
                }

                // 发送推理事件
                let _ = app.emit(
                    "chat-stream",
                    ChatStreamEvent::Reasoning {
                        summary: format!("基于 {} 条证据进行分析", evidence_count),
                    },
                );

                // 发送搜索完成状态
                let _ = app.emit(
                    "chat-stream",
                    ChatStreamEvent::ThinkingStatus {
                        stage: String::from("reasoning"),
                        description: format!("已找到 {} 条相关证据", evidence_count),
                    },
                );

                let evidence_context = format_search_result_for_prompt(&result);
                let enhanced = format!(
                    "{}\n\n在回答前，请参考以下检索到的相关证据（如没有相关证据则直接回答）：\n\n{}",
                    base_system_prompt, evidence_context
                );

                (enhanced, Some(result))
            }
            Err(e) => {
                // 搜索失败但不阻断流程，继续用基础提示词
                eprintln!("[chat_stream] Agentic Search 失败: {}", e);
                let _ = app.emit(
                    "chat-stream",
                    ChatStreamEvent::ThinkingStatus {
                        stage: String::from("search_failed"),
                        description: String::from("检索失败，将直接回答"),
                    },
                );
                (base_system_prompt.to_string(), None)
            }
        }
    } else {
        (base_system_prompt.to_string(), None)
    };

    // 发送生成开始状态
    let _ = app.emit(
        "chat-stream",
        ChatStreamEvent::ThinkingStatus {
            stage: String::from("generating"),
            description: String::from("正在生成回答..."),
        },
    );

    // 使用非流式方式生成（简化实现）
    // 实际流式实现需要使用 ProviderAdapter 的 chat_stream 方法
    let accumulated_content = generate_with_agent(
        pool,
        &config,
        &enhanced_prompt,
        &input.message,
        &app,
        assistant_message_id,
    )
    .await?;

    // 如果有搜索结果，在响应末尾附加引用信息
    if let Some(search_result) = search_context {
        if !search_result.evidence_sources.is_empty() {
            let citation = format!(
                "\n\n---\n参考来源：{}",
                search_result
                    .evidence_sources
                    .iter()
                    .map(|s| format!("[{}]", s.source_type.description()))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let full_content = format!("{}{}", accumulated_content, citation);

            // 更新最终内容到数据库
            ConversationService::update_message_content(pool, assistant_message_id, &full_content)
                .await?;
        }
    }

    // 发送生成完成状态
    let _ = app.emit(
        "chat-stream",
        ChatStreamEvent::ThinkingStatus {
            stage: String::from("complete"),
            description: String::from("回答生成完成"),
        },
    );

    Ok(())
}

/// 使用 Agent 生成响应（非流式，但模拟流式事件）
///
/// TODO: 使用 ProviderAdapter 的 chat_stream 方法实现真正的流式生成
async fn generate_with_agent(
    pool: &SqlitePool,
    config: &crate::models::ai_config::AiConfig,
    system_prompt: &str,
    user_message: &str,
    app: &tauri::AppHandle,
    message_id: &str,
) -> Result<String, AppError> {
    let client = LlmProviderService::create_client(config)?;

    // 获取技能工具
    let skill_tools = build_all_enabled_skill_tools(pool)
        .await
        .unwrap_or_default();

    let response: String = if skill_tools.is_empty() {
        let agent =
            LlmProviderService::create_agent(&client, &config.default_model, system_prompt, 0.7);
        agent
            .prompt(user_message)
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
            .prompt(user_message)
            .await
            .map_err(|error| AppError::ExternalService(format!("AI 对话调用失败：{error}")))?
    };

    // 模拟流式发送 - 按句子分割发送
    let sentences: Vec<&str> = response
        .split_inclusive(&['.', '。', '!', '！', '?', '？', '\n'][..])
        .collect();
    let mut accumulated = String::new();

    for sentence in sentences {
        if !sentence.is_empty() {
            accumulated.push_str(sentence);
            // 发送 chunk 事件到前端
            let _ = app.emit(
                "chat-stream",
                ChatStreamEvent::Chunk {
                    content: sentence.to_string(),
                },
            );
            // 小延迟模拟流式效果
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        }
    }

    // 保存完整响应到数据库
    if !accumulated.is_empty() {
        ConversationService::update_message_content(pool, message_id, &accumulated).await?;
    }

    Ok(accumulated)
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

    teacher_id.ok_or_else(|| AppError::NotFound(String::from("未找到教师档案")))
}

/// 解析工作区路径。
fn resolve_workspace_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|error| {
        AppError::Config(format!(
            "获取应用数据目录失败，无法推导 workspace_path：{}",
            error
        ))
    })?;

    Ok(app_data_dir.join("workspace"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_system_prompt() {
        assert!(get_system_prompt("homeroom").contains("班主任"));
        assert!(get_system_prompt("grading").contains("批改"));
        assert!(get_system_prompt("communication").contains("家校沟通"));
        assert!(get_system_prompt("ops").contains("教务"));
        assert!(get_system_prompt("unknown").contains("PureWorker"));
    }
}

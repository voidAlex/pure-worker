//! AI 通用聊天 IPC 命令模块
//!
//! 提供前端 AI 助手面板的通用对话能力。

use rig::completion::Prompt;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::services::llm_provider::LlmProviderService;

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
    // 校验输入
    if input.message.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("消息内容不能为空")));
    }

    // 获取激活的 AI 配置
    let config = LlmProviderService::get_active_config(&pool).await?;
    let model_name = config.default_model.clone();

    // 创建 LLM 客户端和 Agent
    let client = LlmProviderService::create_client(&config)?;
    let system_prompt = get_system_prompt(&input.agent_role);
    let agent =
        LlmProviderService::create_agent(&client, &config.default_model, system_prompt, 0.7);

    // 调用 LLM
    let response = agent
        .prompt(&input.message)
        .await
        .map_err(|e| AppError::ExternalService(format!("AI 对话调用失败：{e}")))?;

    Ok(ChatResponse {
        content: response,
        model: model_name,
    })
}

//! Provider Adapter 模块
//!
//! 提供统一的 Provider Adapter 模式，支持 OpenAI 兼容 API 和 Anthropic Messages API。
//!
//! # 架构设计
//! - `ProviderAdapter` trait: 定义统一的适配器接口
//! - `ProviderType`: 枚举支持的 Provider 类型
//! - `AdapterFactory`: 工厂模式创建对应适配器
//! - `OpenAiAdapter`: OpenAI 兼容 API 实现
//! - `AnthropicAdapter`: Anthropic Messages API 原生实现

use async_trait::async_trait;
use futures::stream::BoxStream;
use rig::completion::ToolDefinition;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;
use crate::models::ai_config::ModelInfo;

/// Provider 类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum ProviderType {
    /// OpenAI 兼容 API（OpenAI、DeepSeek、Qwen 等）
    OpenAiCompatible,
    /// Anthropic Messages API（Claude）
    AnthropicNative,
}

impl ProviderType {
    /// 从字符串解析 ProviderType，支持 OpenAI 兼容协议（OpenAI/DeepSeek/Qwen/Gemini/自定义）和 Anthropic
    pub fn from_provider_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "openai" | "deepseek" | "qwen" | "gemini" | "google" | "custom" => {
                Some(Self::OpenAiCompatible)
            }
            "anthropic" | "claude" => Some(Self::AnthropicNative),
            _ => None,
        }
    }

    /// 获取 Provider 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "OpenAI Compatible",
            Self::AnthropicNative => "Anthropic Messages API",
        }
    }
}

/// Provider 配置
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider 类型
    pub provider_type: ProviderType,
    /// API 密钥
    pub api_key: String,
    /// Base URL
    pub base_url: String,
    /// 额外请求头
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// 默认模型
    pub default_model: String,
}

/// 聊天消息角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息角色
    pub role: ChatRole,
    /// 消息内容
    pub content: String,
    /// 工具调用ID（用于 tool 角色消息）
    pub tool_call_id: Option<String>,
    /// 工具调用列表（用于 assistant 角色消息）
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 工具参数（JSON 字符串）
    pub arguments: String,
}

/// Provider 适配器 trait
///
/// 定义统一的 LLM Provider 接口，支持对话和流式对话
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// 获取 Provider 类型
    fn provider_type(&self) -> ProviderType;

    /// 执行非流式对话
    ///
    /// # 参数
    /// - `model`: 模型ID
    /// - `messages`: 消息列表
    /// - `tools`: 可选的工具定义列表
    ///
    /// # 返回
    /// 模型的响应文本
    async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<String, AppError>;

    /// 执行流式对话
    ///
    /// # 参数
    /// - `model`: 模型ID
    /// - `messages`: 消息列表
    /// - `tools`: 可选的工具定义列表
    ///
    /// # 返回
    /// 流式响应的字符流
    async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<BoxStream<'static, Result<String, AppError>>, AppError>;

    /// 获取可用模型列表
    ///
    /// # 返回
    /// 模型信息列表
    async fn fetch_models(&self) -> Result<Vec<ModelInfo>, AppError>;
}

/// 适配器工厂
///
/// 根据 ProviderConfig 创建对应的适配器实例
pub struct AdapterFactory;

impl AdapterFactory {
    /// 根据配置创建适配器
    ///
    /// # 参数
    /// - `config`: Provider 配置
    ///
    /// # 返回
    /// 对应类型的适配器实例
    pub fn create(config: &ProviderConfig) -> Box<dyn ProviderAdapter> {
        match config.provider_type {
            ProviderType::OpenAiCompatible => Box::new(openai_adapter::OpenAiAdapter::new(config)),
            ProviderType::AnthropicNative => {
                Box::new(anthropic_adapter::AnthropicAdapter::new(config))
            }
        }
    }
}

/// 判断模型是否为视觉模型
pub fn is_vision_model(model_id: &str) -> bool {
    let vision_keywords = [
        "vision",
        "gpt-4o",
        "claude-3",
        "gemini-pro-vision",
        "gemini-1.5-pro",
        "gemini-1.5-flash",
        "gemini-2.0",
    ];
    let lower = model_id.to_lowercase();
    vision_keywords.iter().any(|kw| lower.contains(kw))
}

/// 获取模型能力元数据
pub fn get_model_capabilities(
    model_id: &str,
    provider_name: &str,
) -> crate::models::ai_config::ModelCapability {
    use crate::models::ai_config::ModelCapability;

    let is_vision = is_vision_model(model_id);
    let lower = model_id.to_lowercase();
    let provider = provider_name.to_lowercase();

    // 基于模型ID推断能力
    let supports_tool_calling = lower.contains("gpt-4")
        || lower.contains("claude-3")
        || lower.contains("gemini-1.5")
        || lower.contains("gemini-2")
        || provider.contains("openai")
        || provider.contains("anthropic");

    let supports_reasoning = lower.contains("o1") || lower.contains("o3") || lower.contains("r1");

    let supports_json_mode = supports_tool_calling;

    // 上下文窗口推断
    let context_window = if lower.contains("32k") {
        32768
    } else if lower.contains("128k") || lower.contains("gpt-4o") {
        128000
    } else if lower.contains("200k") || lower.contains("claude-3") {
        200000
    } else if lower.contains("1m") || lower.contains("gemini") {
        1000000
    } else {
        8192
    };

    // 最大输出token推断
    let max_output_tokens = if lower.contains("mini")
        || lower.contains("flash")
        || lower.contains("claude-3-5")
        || lower.contains("claude-3-opus")
    {
        8192
    } else if lower.contains("claude-3-7") {
        64000
    } else {
        4096
    };

    ModelCapability {
        supports_text_input: true,
        supports_image_input: is_vision,
        supports_audio_input: false, // 暂不默认支持音频
        supports_tool_calling,
        supports_reasoning,
        supports_json_mode,
        context_window,
        max_output_tokens,
    }
}

// 子模块
pub mod anthropic_adapter;
pub mod openai_adapter;

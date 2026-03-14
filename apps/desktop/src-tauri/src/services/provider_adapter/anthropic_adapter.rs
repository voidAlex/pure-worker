//! Anthropic 适配器模块
//!
//! 实现 ProviderAdapter trait，支持 Anthropic Messages API（Claude）
//! 注意：使用原生 Messages API 格式，非 OpenAI 兼容接口

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use rig::completion::ToolDefinition;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::ai_config::{ModelCapability, ModelInfo};
use crate::services::provider_adapter::{
    ChatMessage, ChatRole, ProviderAdapter, ProviderConfig, ProviderType,
};

/// Anthropic API 版本
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic 聊天请求
#[derive(Debug, Serialize)]
struct AnthropicChatRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

/// Anthropic 消息格式
#[derive(Debug, Serialize, Deserialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic 工具定义
#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

/// Anthropic 非流式响应
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicChatResponse {
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

/// Anthropic 内容块
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<serde_json::Value>,
}

/// Anthropic 使用量统计
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic 流式响应事件
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: serde_json::Value },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: AnthropicContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u32,
        delta: AnthropicContentDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: serde_json::Value,
        usage: Option<AnthropicUsage>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
}

/// Anthropic 内容增量
#[derive(Debug, Deserialize)]
struct AnthropicContentDelta {
    #[serde(rename = "type")]
    delta_type: String,
    #[serde(default)]
    text: Option<String>,
}

/// Anthropic 模型列表响应
#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
}

/// Anthropic 模型
#[derive(Debug, Deserialize)]
struct AnthropicModel {
    id: String,
    #[serde(default)]
    display_name: String,
}

/// Anthropic 适配器
pub struct AnthropicAdapter {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl AnthropicAdapter {
    /// 创建新的 Anthropic 适配器实例
    pub fn new(config: &ProviderConfig) -> Self {
        Self {
            config: config.clone(),
            client: reqwest::Client::new(),
        }
    }

    /// 将内部消息格式转换为 Anthropic 格式
    fn convert_messages(messages: Vec<ChatMessage>) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system_message: Option<String> = None;
        let mut anthropic_messages: Vec<AnthropicMessage> = Vec::new();

        for msg in messages {
            match msg.role {
                ChatRole::System => {
                    // Anthropic 使用顶级 system 字段
                    system_message = Some(msg.content);
                }
                ChatRole::User => {
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: msg.content,
                    });
                }
                ChatRole::Assistant => {
                    anthropic_messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: msg.content,
                    });
                }
                ChatRole::Tool => {
                    // Anthropic 工具结果作为 user 角色消息，带有 tool_result 格式
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: format!("<tool_result>{}</tool_result>", msg.content),
                    });
                }
            }
        }

        (system_message, anthropic_messages)
    }

    /// 将 ToolDefinition 转换为 Anthropic 格式
    fn convert_tools(tools: Vec<ToolDefinition>) -> Vec<AnthropicTool> {
        tools
            .into_iter()
            .map(|tool| AnthropicTool {
                name: tool.name,
                description: tool.description,
                input_schema: tool.parameters,
            })
            .collect()
    }

    /// 构建请求 URL
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), endpoint)
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            self.config.api_key.parse().expect("无效的 API Key"),
        );
        headers.insert(
            "anthropic-version",
            ANTHROPIC_VERSION.parse().expect("无效的版本号"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("无效的内容类型"),
        );

        // 添加自定义请求头
        if let Some(custom_headers) = &self.config.headers {
            for (key, value) in custom_headers {
                if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(header_value) = value.parse() {
                        headers.insert(header_name, header_value);
                    }
                }
            }
        }

        headers
    }
}

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    fn provider_type(&self) -> ProviderType {
        ProviderType::AnthropicNative
    }

    async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<String, AppError> {
        let url = self.build_url("/v1/messages");
        let (system, anthropic_messages) = Self::convert_messages(messages);
        let anthropic_tools = tools.map(Self::convert_tools);

        let request_body = AnthropicChatRequest {
            model: model.to_string(),
            messages: anthropic_messages,
            system,
            max_tokens: 4096,
            tools: anthropic_tools,
            tool_choice: None,
            stream: Some(false),
            temperature: Some(0.7),
        };

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("请求发送失败：{}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(AppError::ExternalService(format!(
                "API 请求失败（{}）：{}",
                status, error_text
            )));
        }

        let chat_response: AnthropicChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("响应解析失败：{}", e)))?;

        // 提取文本内容
        let mut result = String::new();
        for block in chat_response.content {
            if block.content_type == "text" {
                if let Some(text) = block.text {
                    result.push_str(&text);
                }
            }
        }

        if result.is_empty() {
            Err(AppError::ExternalService(String::from("API 返回空响应")))
        } else {
            Ok(result)
        }
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<BoxStream<'static, Result<String, AppError>>, AppError> {
        let url = self.build_url("/v1/messages");
        let (system, anthropic_messages) = Self::convert_messages(messages);
        let anthropic_tools = tools.map(Self::convert_tools);

        let request_body = AnthropicChatRequest {
            model: model.to_string(),
            messages: anthropic_messages,
            system,
            max_tokens: 4096,
            tools: anthropic_tools,
            tool_choice: None,
            stream: Some(true),
            temperature: Some(0.7),
        };

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("请求发送失败：{}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(AppError::ExternalService(format!(
                "API 请求失败（{}）：{}",
                status, error_text
            )));
        }

        let stream = response
            .bytes_stream()
            .filter_map(|result| async move {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let mut content_parts = Vec::new();

                        for line in text.lines() {
                            let line = line.trim();
                            if line.is_empty() {
                                continue;
                            }

                            if let Some(event_json) = line.strip_prefix("data: ") {
                                match serde_json::from_str::<AnthropicStreamEvent>(event_json) {
                                    Ok(AnthropicStreamEvent::ContentBlockDelta {
                                        delta, ..
                                    }) => {
                                        if delta.delta_type == "text_delta" {
                                            if let Some(text) = delta.text {
                                                if !text.is_empty() {
                                                    content_parts.push(text);
                                                }
                                            }
                                        }
                                    }
                                    _ => continue,
                                }
                            }
                        }

                        if content_parts.is_empty() {
                            None
                        } else {
                            Some(Ok(content_parts.join("")))
                        }
                    }
                    Err(e) => Some(Err(AppError::ExternalService(format!("流读取失败：{}", e)))),
                }
            })
            .boxed();

        Ok(stream)
    }

    async fn fetch_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        let url = self.build_url("/v1/models");

        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("请求模型列表失败：{}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(AppError::ExternalService(format!(
                "获取模型列表失败（{}）：{}",
                status, error_text
            )));
        }

        let models_response: AnthropicModelsResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("解析模型列表失败：{}", e)))?;

        let models = models_response
            .data
            .into_iter()
            .map(|model| {
                let model_id = model.id.clone();
                let capabilities = get_model_capabilities_anthropic(&model_id);
                let name = if model.display_name.is_empty() {
                    model_id.clone()
                } else {
                    model.display_name
                };

                ModelInfo {
                    id: model_id,
                    name,
                    is_vision: capabilities.supports_image_input,
                    capabilities,
                }
            })
            .collect();

        Ok(models)
    }
}

/// 获取 Anthropic 模型的能力元数据
fn get_model_capabilities_anthropic(model_id: &str) -> ModelCapability {
    let model_id_lower = model_id.to_lowercase();

    if model_id_lower.contains("claude-3-7") {
        ModelCapability {
            supports_text_input: true,
            supports_image_input: true,
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: true,
            supports_json_mode: true,
            context_window: 200_000,
            max_output_tokens: 64_000,
        }
    } else if model_id_lower.contains("claude-3-5") {
        ModelCapability {
            supports_text_input: true,
            supports_image_input: true,
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: true,
            supports_json_mode: true,
            context_window: 200_000,
            max_output_tokens: 8192,
        }
    } else if model_id_lower.contains("claude-3-opus") {
        ModelCapability {
            supports_text_input: true,
            supports_image_input: true,
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: true,
            supports_json_mode: true,
            context_window: 200_000,
            max_output_tokens: 4096,
        }
    } else if model_id_lower.contains("claude-3-sonnet")
        || model_id_lower.contains("claude-3-haiku")
    {
        ModelCapability {
            supports_text_input: true,
            supports_image_input: model_id_lower.contains("sonnet"),
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: false,
            supports_json_mode: true,
            context_window: 200_000,
            max_output_tokens: 4096,
        }
    } else {
        // 默认 Claude-3 能力
        ModelCapability {
            supports_text_input: true,
            supports_image_input: true,
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: model_id_lower.contains("sonnet")
                || model_id_lower.contains("opus"),
            supports_json_mode: true,
            context_window: 200_000,
            max_output_tokens: 4096,
        }
    }
}

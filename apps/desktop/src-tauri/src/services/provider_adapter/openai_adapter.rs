//! OpenAI 兼容适配器模块
//!
//! 实现 ProviderAdapter trait，支持 OpenAI 兼容 API（包括 OpenAI、DeepSeek、Qwen 等）

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use rig::completion::ToolDefinition;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::ai_config::{ModelCapability, ModelInfo};
use crate::services::provider_adapter::{
    ChatMessage, ChatRole, ProviderAdapter, ProviderConfig, ProviderType,
};

/// OpenAI 聊天请求
#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

/// OpenAI 消息格式
#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// OpenAI 工具定义
#[derive(Debug, Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunction,
}

/// OpenAI 函数定义
#[derive(Debug, Serialize)]
struct OpenAiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// OpenAI 工具调用
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunctionCall,
}

/// OpenAI 函数调用
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

/// OpenAI 流式响应块
#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

/// OpenAI 流式选择
#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
}

/// OpenAI 流式增量
#[derive(Debug, Deserialize, Default)]
struct OpenAiStreamDelta {
    #[serde(default)]
    content: Option<String>,
}

/// OpenAI 非流式响应
#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

/// OpenAI 选择
#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

/// OpenAI 响应消息
#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

/// OpenAI 模型列表响应
#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

/// OpenAI 模型
#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
}

/// OpenAI 兼容适配器
pub struct OpenAiAdapter {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl OpenAiAdapter {
    /// 创建新的 OpenAI 适配器实例
    pub fn new(config: &ProviderConfig) -> Self {
        Self {
            config: config.clone(),
            client: reqwest::Client::new(),
        }
    }

    /// 将内部消息格式转换为 OpenAI 格式
    fn convert_messages(messages: Vec<ChatMessage>) -> Vec<OpenAiMessage> {
        messages
            .into_iter()
            .map(|msg| OpenAiMessage {
                role: match msg.role {
                    ChatRole::System => "system".to_string(),
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                    ChatRole::Tool => "tool".to_string(),
                },
                content: msg.content,
                tool_call_id: msg.tool_call_id,
                tool_calls: msg.tool_calls.map(|calls| {
                    calls
                        .into_iter()
                        .map(|call| OpenAiToolCall {
                            id: call.id,
                            tool_type: "function".to_string(),
                            function: OpenAiFunctionCall {
                                name: call.name,
                                arguments: call.arguments,
                            },
                        })
                        .collect()
                }),
            })
            .collect()
    }

    /// 将 ToolDefinition 转换为 OpenAI 格式
    ///
    /// 注意：OpenAI 要求工具名称必须符合正则表达式 ^[a-zA-Z0-9_-]+$（仅允许字母、数字、下划线和连字符）
    /// 如果工具名称包含点号（如 "math.compute"），需要将其替换为下划线
    fn convert_tools(tools: Vec<ToolDefinition>) -> Vec<OpenAiTool> {
        tools
            .into_iter()
            .map(|tool| {
                // 规范化工具名称：将点号替换为下划线，以符合 OpenAI 命名规范
                let normalized_name = tool.name.replace('.', "_");
                OpenAiTool {
                    tool_type: "function".to_string(),
                    function: OpenAiFunction {
                        name: normalized_name,
                        description: tool.description,
                        parameters: tool.parameters,
                    },
                }
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
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.config.api_key)
                .parse()
                .expect("无效的 API Key"),
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
impl ProviderAdapter for OpenAiAdapter {
    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAiCompatible
    }

    async fn chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<String, AppError> {
        let url = self.build_url("/v1/chat/completions");
        let openai_messages = Self::convert_messages(messages);
        let openai_tools = tools.map(Self::convert_tools);

        let request_body = OpenAiChatRequest {
            model: model.to_string(),
            messages: openai_messages,
            tools: openai_tools,
            tool_choice: None,
            stream: false,
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

        let chat_response: OpenAiChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("响应解析失败：{}", e)))?;

        if let Some(choice) = chat_response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(AppError::ExternalService(String::from("API 返回空响应")))
        }
    }

    async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<BoxStream<'static, Result<String, AppError>>, AppError> {
        let url = self.build_url("/v1/chat/completions");
        let openai_messages = Self::convert_messages(messages);
        let openai_tools = tools.map(Self::convert_tools);

        let request_body = OpenAiChatRequest {
            model: model.to_string(),
            messages: openai_messages,
            tools: openai_tools,
            tool_choice: None,
            stream: true,
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
                            if line.is_empty() || line.starts_with(":") {
                                continue;
                            }

                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    continue;
                                }

                                match serde_json::from_str::<OpenAiStreamChunk>(data) {
                                    Ok(chunk) => {
                                        if let Some(choice) = chunk.choices.first() {
                                            if let Some(content) = &choice.delta.content {
                                                if !content.is_empty() {
                                                    content_parts.push(content.clone());
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => continue,
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

        let models_response: OpenAiModelsResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("解析模型列表失败：{}", e)))?;

        let models = models_response
            .data
            .into_iter()
            .map(|model| {
                let model_id = model.id.clone();
                let capabilities = get_model_capabilities_openai(&model_id);

                ModelInfo {
                    id: model_id.clone(),
                    name: model_id.clone(),
                    is_vision: capabilities.supports_image_input,
                    capabilities,
                }
            })
            .collect();

        Ok(models)
    }
}

/// 获取 OpenAI 兼容模型的能力元数据
fn get_model_capabilities_openai(model_id: &str) -> ModelCapability {
    let model_id_lower = model_id.to_lowercase();

    // GPT-4 系列
    if model_id_lower.starts_with("gpt-4o") {
        ModelCapability {
            supports_text_input: true,
            supports_image_input: true,
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: false,
            supports_json_mode: true,
            context_window: 128_000,
            max_output_tokens: 16_384,
        }
    } else if model_id_lower.starts_with("gpt-4") {
        ModelCapability {
            supports_text_input: true,
            supports_image_input: model_id_lower.contains("vision")
                || model_id_lower.contains("turbo"),
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: model_id_lower.contains("o1") || model_id_lower.contains("o3"),
            supports_json_mode: true,
            context_window: if model_id_lower.contains("32k") {
                32_768
            } else if model_id_lower.contains("128k") || model_id_lower.contains("turbo") {
                128_000
            } else {
                8_192
            },
            max_output_tokens: 4096,
        }
    } else if model_id_lower.starts_with("gpt-3.5") {
        ModelCapability {
            supports_text_input: true,
            supports_image_input: false,
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: false,
            supports_json_mode: true,
            context_window: if model_id_lower.contains("16k") {
                16_384
            } else {
                4096
            },
            max_output_tokens: 4096,
        }
    } else if model_id_lower.contains("o1") || model_id_lower.contains("o3") {
        // o1/o3 推理模型
        ModelCapability {
            supports_text_input: true,
            supports_image_input: true,
            supports_audio_input: false,
            supports_tool_calling: true,
            supports_reasoning: true,
            supports_json_mode: true,
            context_window: 128_000,
            max_output_tokens: if model_id_lower.contains("o1") {
                32_768
            } else {
                16_384
            },
        }
    } else {
        // 默认能力
        ModelCapability {
            supports_text_input: true,
            supports_image_input: model_id_lower.contains("vision"),
            supports_audio_input: false,
            supports_tool_calling: model_id_lower.contains("tool")
                || model_id_lower.contains("function"),
            supports_reasoning: false,
            supports_json_mode: true,
            context_window: 8192,
            max_output_tokens: 4096,
        }
    }
}

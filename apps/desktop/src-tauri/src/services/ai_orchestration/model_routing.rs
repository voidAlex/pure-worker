//! 模型路由服务
//!
//! 根据任务能力需求（文本/多模态/工具/推理）选择模型，并输出可审计路由轨迹。

use crate::models::ai_config::{AiConfig, ModelCapability};
use crate::services::provider_adapter::get_model_capabilities;

use super::error::{OrchestrationError, OrchestrationResult};

/// 路由能力需求
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingCapability {
    Text,
    Vision,
    Tool,
    Reasoning,
}

impl RoutingCapability {
    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Vision => "vision",
            Self::Tool => "tool",
            Self::Reasoning => "reasoning",
        }
    }
}

/// 路由轨迹
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingTrace {
    pub requested_capability: String,
    pub candidate_model: String,
    pub selected_model: String,
    pub fallback_chain: Vec<String>,
}

/// 路由结果
#[derive(Debug, Clone)]
pub struct SelectedModel {
    pub provider_id: String,
    pub model_id: String,
    pub capability: ModelCapability,
    pub fallback_used: bool,
    pub trace: RoutingTrace,
}

/// 模型路由服务
pub struct ModelRoutingService;

impl ModelRoutingService {
    /// 选择模型
    pub fn select_model(
        config: &AiConfig,
        capability: RoutingCapability,
        profile_specified_model: Option<&str>,
        allow_text_to_vision_fallback: bool,
    ) -> OrchestrationResult<SelectedModel> {
        if let Some(model_id) = profile_specified_model {
            return Self::select_explicit(config, capability, model_id);
        }

        let (candidate, fallback_chain) =
            Self::resolve_candidate_models(config, capability, allow_text_to_vision_fallback)?;

        let capability_snapshot = get_model_capabilities(&candidate, &config.provider_name);
        Self::ensure_capability(capability, &candidate, &capability_snapshot)?;

        Ok(SelectedModel {
            provider_id: config.id.clone(),
            model_id: candidate.clone(),
            capability: capability_snapshot,
            fallback_used: fallback_chain.len() > 1,
            trace: RoutingTrace {
                requested_capability: capability.as_str().to_string(),
                candidate_model: fallback_chain
                    .first()
                    .cloned()
                    .unwrap_or_else(|| candidate.clone()),
                selected_model: candidate,
                fallback_chain,
            },
        })
    }

    fn select_explicit(
        config: &AiConfig,
        capability: RoutingCapability,
        model_id: &str,
    ) -> OrchestrationResult<SelectedModel> {
        let snapshot = get_model_capabilities(model_id, &config.provider_name);
        Self::ensure_capability(capability, model_id, &snapshot)?;

        Ok(SelectedModel {
            provider_id: config.id.clone(),
            model_id: model_id.to_string(),
            capability: snapshot,
            fallback_used: false,
            trace: RoutingTrace {
                requested_capability: capability.as_str().to_string(),
                candidate_model: model_id.to_string(),
                selected_model: model_id.to_string(),
                fallback_chain: vec![model_id.to_string()],
            },
        })
    }

    fn resolve_candidate_models(
        config: &AiConfig,
        capability: RoutingCapability,
        allow_text_to_vision_fallback: bool,
    ) -> OrchestrationResult<(String, Vec<String>)> {
        match capability {
            RoutingCapability::Text => {
                if let Some(model_id) = non_empty_opt(config.default_text_model.as_deref()) {
                    return Ok((model_id.to_string(), vec![model_id.to_string()]));
                }

                if allow_text_to_vision_fallback {
                    if let Some(model_id) = non_empty_opt(config.default_vision_model.as_deref()) {
                        return Ok((
                            model_id.to_string(),
                            vec![String::from("default_text_model"), model_id.to_string()],
                        ));
                    }
                }

                Ok((
                    config.default_model.clone(),
                    vec![
                        String::from("default_text_model"),
                        config.default_model.clone(),
                    ],
                ))
            }
            RoutingCapability::Vision => {
                if let Some(model_id) = non_empty_opt(config.default_vision_model.as_deref()) {
                    return Ok((model_id.to_string(), vec![model_id.to_string()]));
                }

                Ok((
                    config.default_model.clone(),
                    vec![
                        String::from("default_vision_model"),
                        config.default_model.clone(),
                    ],
                ))
            }
            RoutingCapability::Tool => {
                if let Some(model_id) = non_empty_opt(config.default_tool_model.as_deref()) {
                    return Ok((model_id.to_string(), vec![model_id.to_string()]));
                }

                Ok((
                    config.default_model.clone(),
                    vec![
                        String::from("default_tool_model"),
                        config.default_model.clone(),
                    ],
                ))
            }
            RoutingCapability::Reasoning => {
                if let Some(model_id) = non_empty_opt(config.default_reasoning_model.as_deref()) {
                    return Ok((model_id.to_string(), vec![model_id.to_string()]));
                }

                Ok((
                    config.default_model.clone(),
                    vec![
                        String::from("default_reasoning_model"),
                        config.default_model.clone(),
                    ],
                ))
            }
        }
    }

    fn ensure_capability(
        capability: RoutingCapability,
        model_id: &str,
        capability_snapshot: &ModelCapability,
    ) -> OrchestrationResult<()> {
        let supported = match capability {
            RoutingCapability::Text => capability_snapshot.supports_text_input,
            RoutingCapability::Vision => capability_snapshot.supports_image_input,
            RoutingCapability::Tool => capability_snapshot.supports_tool_calling,
            RoutingCapability::Reasoning => capability_snapshot.supports_reasoning,
        };

        if supported {
            return Ok(());
        }

        Err(OrchestrationError::ModelCapabilityInsufficient(format!(
            "模型 {} 不满足 {} 能力要求",
            model_id,
            capability.as_str()
        )))
    }
}

fn non_empty_opt(value: Option<&str>) -> Option<&str> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> AiConfig {
        AiConfig {
            id: String::from("cfg-openai"),
            provider_name: String::from("openai"),
            display_name: String::from("OpenAI"),
            base_url: String::from("https://api.openai.com/v1"),
            api_key_encrypted: String::from("encrypted"),
            default_model: String::from("gpt-4o"),
            default_text_model: Some(String::from("gpt-4o-mini")),
            default_vision_model: Some(String::from("gpt-4o")),
            default_tool_model: Some(String::from("gpt-4o")),
            default_reasoning_model: Some(String::from("o3-mini")),
            is_active: 1,
            config_json: None,
            is_deleted: 0,
            created_at: String::from("2026-03-22T00:00:00Z"),
            updated_at: String::from("2026-03-22T00:00:00Z"),
        }
    }

    /// profile 指定模型优先
    #[test]
    fn test_profile_specified_model_wins() {
        let config = base_config();
        let selected = ModelRoutingService::select_model(
            &config,
            RoutingCapability::Vision,
            Some("gpt-4o"),
            false,
        )
        .expect("routing should succeed");

        assert_eq!(selected.model_id, "gpt-4o");
        assert!(!selected.fallback_used);
    }

    /// 能力映射到正确默认模型字段
    #[test]
    fn test_capability_maps_to_default_field() {
        let config = base_config();

        let text = ModelRoutingService::select_model(&config, RoutingCapability::Text, None, false)
            .expect("text routing should succeed");
        let vision =
            ModelRoutingService::select_model(&config, RoutingCapability::Vision, None, false)
                .expect("vision routing should succeed");

        assert_eq!(text.model_id, "gpt-4o-mini");
        assert_eq!(vision.model_id, "gpt-4o");
    }

    /// 缺失专用模型时，在同能力层回落 default_model
    #[test]
    fn test_fallback_within_same_capability_tier() {
        let mut config = base_config();
        config.default_tool_model = None;

        let selected =
            ModelRoutingService::select_model(&config, RoutingCapability::Tool, None, false)
                .expect("tool routing should succeed");

        assert_eq!(selected.model_id, "gpt-4o");
        assert!(selected.fallback_used);
    }

    /// 能力不足时返回显式错误
    #[test]
    fn test_capability_insufficient_returns_error() {
        let mut config = base_config();
        config.default_model = String::from("gpt-3.5-turbo");
        config.default_vision_model = None;

        let result =
            ModelRoutingService::select_model(&config, RoutingCapability::Vision, None, false);
        assert!(matches!(
            result,
            Err(OrchestrationError::ModelCapabilityInsufficient(_))
        ));
    }

    /// 多模态请求绝不回落 text-only
    #[test]
    fn test_multimodal_never_fallback_to_text_only() {
        let mut config = base_config();
        config.default_model = String::from("gpt-3.5-turbo");
        config.default_vision_model = None;

        let result =
            ModelRoutingService::select_model(&config, RoutingCapability::Vision, None, false);
        assert!(result.is_err());
    }

    /// text 请求默认走 text 模型，且仅在显式允许时使用 vision fallback
    #[test]
    fn test_text_only_path_and_optional_vision_fallback() {
        let mut config = base_config();
        config.default_text_model = None;
        config.default_model = String::from("gpt-3.5-turbo");
        config.default_vision_model = Some(String::from("gpt-4o"));

        let strict =
            ModelRoutingService::select_model(&config, RoutingCapability::Text, None, false)
                .expect("strict text route should succeed");
        assert_eq!(strict.model_id, "gpt-3.5-turbo");

        let with_fallback =
            ModelRoutingService::select_model(&config, RoutingCapability::Text, None, true)
                .expect("fallback text route should succeed");
        assert_eq!(with_fallback.model_id, "gpt-4o");
    }
}

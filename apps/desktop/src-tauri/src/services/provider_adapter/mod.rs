//! Provider Adapter 模块
//!
//! 提供统一的 LLM Provider 能力检测（WP-AI-004, WP-AI-005）。
//! 当前版本优先实现模型能力元数据支持。

use crate::models::ai_config::ModelCapability;

/// 根据模型ID和供应商获取能力元数据（WP-AI-005）。
pub fn get_model_capabilities(model_id: &str, provider: &str) -> ModelCapability {
    let model_id_lower = model_id.to_lowercase();

    match provider {
        "anthropic" => ModelCapability {
            supports_text_input: true,
            supports_image_input: model_id_lower.contains("vision")
                || model_id_lower.contains("opus")
                || model_id_lower.contains("sonnet"),
            supports_audio_input: false,
            supports_tool_calling: model_id_lower.contains("claude-3"),
            supports_reasoning: model_id_lower.contains("claude-3-5")
                || model_id_lower.contains("claude-3-7"),
            supports_json_mode: model_id_lower.contains("claude-3"),
            context_window: 200_000,
            max_output_tokens: if model_id_lower.contains("opus") {
                4096
            } else {
                8192
            },
        },
        "gemini" => ModelCapability {
            supports_text_input: true,
            supports_image_input: model_id_lower.contains("vision")
                || model_id_lower.contains("pro")
                || model_id_lower.contains("flash"),
            supports_audio_input: false,
            supports_tool_calling: model_id_lower.contains("pro")
                || model_id_lower.contains("flash"),
            supports_reasoning: model_id_lower.contains("thinking"),
            supports_json_mode: model_id_lower.contains("pro") || model_id_lower.contains("flash"),
            context_window: if model_id_lower.contains("1.5") {
                1_000_000
            } else {
                32_768
            },
            max_output_tokens: if model_id_lower.contains("1.5") {
                8192
            } else {
                2048
            },
        },
        _ => {
            // OpenAI 兼容接口
            ModelCapability {
                supports_text_input: true,
                supports_image_input: model_id_lower.starts_with("gpt-4o")
                    || model_id_lower.contains("vision")
                    || model_id_lower.contains("turbo"),
                supports_audio_input: false,
                supports_tool_calling: model_id_lower.starts_with("gpt-4")
                    || model_id_lower.starts_with("gpt-3.5"),
                supports_reasoning: model_id_lower.contains("o1") || model_id_lower.contains("o3"),
                supports_json_mode: model_id_lower.starts_with("gpt-4")
                    || model_id_lower.starts_with("gpt-3.5"),
                context_window: if model_id_lower.contains("o1") || model_id_lower.contains("128k")
                {
                    128_000
                } else {
                    8_192
                },
                max_output_tokens: if model_id_lower.contains("o1") {
                    32_768
                } else if model_id_lower.starts_with("gpt-4o") {
                    16_384
                } else {
                    4096
                },
            }
        }
    }
}

/// 判断模型是否支持视觉/多模态（向后兼容）。
pub fn is_vision_model(model_id: &str) -> bool {
    let prefixes = [
        "gpt-4o",
        "gpt-4o-mini",
        "gpt-5",
        "gpt-4-vision",
        "gpt-4-turbo",
        "claude-3-opus",
        "claude-3-5-sonnet",
        "claude-3-5-haiku",
        "claude-3-haiku",
        "claude-sonnet-4",
        "claude-opus-4",
        "gemini-1.5-pro",
        "gemini-1.5-flash",
        "gemini-2.0",
        "gemini-pro-vision",
        "qwen-vl",
        "qwen2-vl",
    ];

    let model_id_lower = model_id.to_lowercase();
    prefixes
        .iter()
        .any(|prefix| model_id_lower.starts_with(prefix))
}

//! ProviderCatalog 与 LoaderMatrix
//!
//! 统一维护 provider 注册状态、模型目录与 loader 类型映射。

use std::collections::{HashMap, HashSet};

use crate::models::ai_config::{AiConfig, ModelCapability};
use crate::services::provider_adapter::{get_model_capabilities, is_vision_model, ProviderType};

/// Provider 加载器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderLoaderKind {
    OpenAiCompatible,
    AnthropicNative,
    CustomCompatible,
}

/// 模型目录条目
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogModel {
    pub provider_id: String,
    pub model_id: String,
    pub input_modalities: Vec<String>,
    pub supports_text_input: bool,
    pub supports_image_input: bool,
    pub supports_tool_calling: bool,
    pub supports_reasoning: bool,
    pub context_window: u32,
    pub max_output_tokens: u32,
}

/// Provider 注册项
#[derive(Debug, Clone)]
pub struct ProviderCatalogEntry {
    pub provider_id: String,
    pub provider_name: String,
    pub enabled: bool,
    pub healthy: bool,
    pub loader_kind: ProviderLoaderKind,
    pub models: Vec<CatalogModel>,
}

/// Provider 目录
#[derive(Debug, Clone, Default)]
pub struct ProviderCatalog {
    providers: HashMap<String, ProviderCatalogEntry>,
}

impl ProviderCatalog {
    /// 从 AI 配置构建 provider 目录
    pub fn from_configs(configs: &[AiConfig]) -> Self {
        let mut providers = HashMap::new();

        for config in configs {
            let entry = ProviderCatalogEntry {
                provider_id: config.id.clone(),
                provider_name: config.provider_name.clone(),
                enabled: config.is_active == 1,
                healthy: true,
                loader_kind: loader_kind_from_provider(&config.provider_name),
                models: model_catalog_from_config(config),
            };
            providers.insert(entry.provider_id.clone(), entry);
        }

        Self { providers }
    }

    /// 获取 provider
    pub fn get_provider(&self, provider_id: &str) -> Option<&ProviderCatalogEntry> {
        self.providers.get(provider_id)
    }

    /// 获取指定 provider 下模型
    pub fn get_model(&self, provider_id: &str, model_id: &str) -> Option<&CatalogModel> {
        self.providers.get(provider_id).and_then(|provider| {
            provider
                .models
                .iter()
                .find(|model| model.model_id == model_id)
        })
    }

    /// 列出健康且启用的 provider
    pub fn list_healthy_enabled(&self) -> Vec<&ProviderCatalogEntry> {
        self.providers
            .values()
            .filter(|provider| provider.enabled && provider.healthy)
            .collect()
    }
}

fn model_catalog_from_config(config: &AiConfig) -> Vec<CatalogModel> {
    let mut seen = HashSet::new();
    let mut models = Vec::new();

    for model_id in fallback_model_candidates(config) {
        if model_id.is_empty() || !seen.insert(model_id.clone()) {
            continue;
        }
        models.push(catalog_model_from_id(
            &config.id,
            &config.provider_name,
            &model_id,
        ));
    }

    models
}

fn fallback_model_candidates(config: &AiConfig) -> Vec<String> {
    let mut models = vec![config.default_model.clone()];
    if let Some(model) = config.default_text_model.clone() {
        models.push(model);
    }
    if let Some(model) = config.default_vision_model.clone() {
        models.push(model);
    }
    if let Some(model) = config.default_tool_model.clone() {
        models.push(model);
    }
    if let Some(model) = config.default_reasoning_model.clone() {
        models.push(model);
    }
    models
}

fn catalog_model_from_id(provider_id: &str, provider_name: &str, model_id: &str) -> CatalogModel {
    let capabilities = get_model_capabilities(model_id, provider_name);
    let input_modalities = build_modalities(&capabilities, model_id);
    CatalogModel {
        provider_id: provider_id.to_string(),
        model_id: model_id.to_string(),
        input_modalities,
        supports_text_input: capabilities.supports_text_input,
        supports_image_input: capabilities.supports_image_input,
        supports_tool_calling: capabilities.supports_tool_calling,
        supports_reasoning: capabilities.supports_reasoning,
        context_window: capabilities.context_window,
        max_output_tokens: capabilities.max_output_tokens,
    }
}

fn build_modalities(capability: &ModelCapability, model_id: &str) -> Vec<String> {
    let mut modalities = vec![String::from("text")];
    if capability.supports_image_input || is_vision_model(model_id) {
        modalities.push(String::from("image"));
    }
    if capability.supports_audio_input {
        modalities.push(String::from("audio"));
    }
    modalities
}

fn loader_kind_from_provider(provider_name: &str) -> ProviderLoaderKind {
    match ProviderType::from_provider_name(provider_name) {
        Some(ProviderType::OpenAiCompatible) => {
            if provider_name.eq_ignore_ascii_case("custom") {
                ProviderLoaderKind::CustomCompatible
            } else {
                ProviderLoaderKind::OpenAiCompatible
            }
        }
        Some(ProviderType::AnthropicNative) => ProviderLoaderKind::AnthropicNative,
        None => ProviderLoaderKind::CustomCompatible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_config() -> AiConfig {
        AiConfig {
            id: String::from("cfg-openai"),
            provider_name: String::from("openai"),
            display_name: String::from("OpenAI"),
            base_url: String::from("https://api.openai.com/v1"),
            api_key_encrypted: String::from("encrypted"),
            default_model: String::from("gpt-4o"),
            default_text_model: Some(String::from("gpt-4o-mini")),
            default_vision_model: Some(String::from("gpt-4o")),
            default_tool_model: Some(String::from("gpt-4o-mini")),
            default_reasoning_model: Some(String::from("o3-mini")),
            is_active: 1,
            config_json: None,
            is_deleted: 0,
            created_at: String::from("2026-03-22T00:00:00Z"),
            updated_at: String::from("2026-03-22T00:00:00Z"),
        }
    }

    /// 验证 provider 目录包含启用/健康状态与 loader 类型
    #[test]
    fn test_provider_registry_with_health_and_loader() {
        let catalog = ProviderCatalog::from_configs(&[build_config()]);
        let provider = catalog
            .get_provider("cfg-openai")
            .expect("provider should exist");

        assert!(provider.enabled);
        assert!(provider.healthy);
        assert_eq!(provider.loader_kind, ProviderLoaderKind::OpenAiCompatible);
    }

    /// 验证模型目录至少包含关键 capability 字段
    #[test]
    fn test_model_catalog_contains_capabilities() {
        let catalog = ProviderCatalog::from_configs(&[build_config()]);
        let model = catalog
            .get_model("cfg-openai", "gpt-4o")
            .expect("model should exist");

        assert!(model.supports_text_input);
        assert!(model.input_modalities.iter().any(|mode| mode == "text"));
        assert!(model.context_window > 0);
        assert!(model.max_output_tokens > 0);
    }

    /// 验证 fallback 模型合并逻辑会去重
    #[test]
    fn test_model_catalog_merge_and_dedup() {
        let config = build_config();
        let catalog = ProviderCatalog::from_configs(&[config]);
        let provider = catalog
            .get_provider("cfg-openai")
            .expect("provider should exist");

        let mut ids: Vec<&str> = provider
            .models
            .iter()
            .map(|model| model.model_id.as_str())
            .collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), provider.models.len());
    }
}

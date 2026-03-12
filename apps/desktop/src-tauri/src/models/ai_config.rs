//! AI 配置数据模型
//!
//! 定义 AI Provider 配置结构、前端安全视图及命令输入类型。

use serde::{Deserialize, Serialize};
use specta::Type;

/// AI Provider 配置记录。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct AiConfig {
    pub id: String,
    pub provider_name: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key_encrypted: String,
    pub default_model: String,
    pub is_active: i32,
    pub config_json: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 前端展示用 AI 配置（隐藏密钥）。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AiConfigSafe {
    pub id: String,
    pub provider_name: String,
    pub display_name: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub default_model: String,
    pub is_active: i32,
    pub config_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建 AI 配置输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateAiConfigInput {
    pub provider_name: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    pub is_active: Option<bool>,
    pub config_json: Option<String>,
}

/// 更新 AI 配置输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdateAiConfigInput {
    pub id: String,
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub default_model: Option<String>,
    pub is_active: Option<bool>,
    pub config_json: Option<String>,
}

/// 模型信息。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ModelInfo {
    /// 模型ID（如 gpt-4o, claude-3-5-sonnet-20241022）。
    pub id: String,
    /// 模型显示名称。
    pub name: String,
    /// 是否支持视觉/多模态。
    pub is_vision: bool,
}

/// 供应商预设配置。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProviderPreset {
    /// 供应商标识。
    pub name: String,
    /// 供应商显示名称。
    pub display_name: String,
    /// 供应商默认 Base URL。
    pub base_url: String,
}

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

//! AI 参数预设数据模型
//!
//! 定义参数预设记录及创建/更新输入结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// AI 参数预设记录。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct AiParamPreset {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub temperature: f64,
    pub top_p: Option<f64>,
    pub max_tokens: Option<i32>,
    pub is_default: i32,
    pub is_active: i32,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl AiParamPreset {
    /// 返回默认平衡预设（作为兜底温度来源）。
    pub fn default_balanced() -> Self {
        Self {
            id: String::from("balanced-fallback"),
            name: String::from("balanced"),
            display_name: String::from("平衡模式"),
            temperature: 0.7,
            top_p: Some(0.9),
            max_tokens: Some(2048),
            is_default: 1,
            is_active: 1,
            is_deleted: 0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

/// 创建参数预设输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreatePresetInput {
    pub name: String,
    pub display_name: String,
    pub temperature: f64,
    pub top_p: Option<f64>,
    pub max_tokens: Option<i32>,
    pub is_default: Option<bool>,
    pub is_active: Option<bool>,
}

/// 更新参数预设输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdatePresetInput {
    pub id: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<i32>,
    pub is_default: Option<bool>,
    pub is_active: Option<bool>,
}

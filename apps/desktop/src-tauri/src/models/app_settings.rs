//! 应用设置数据模型
//!
//! 定义应用设置记录及创建/更新输入结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 应用设置记录。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct AppSetting {
    pub id: String,
    pub key: String,
    pub value: String,
    pub category: String,
    pub description: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建设置输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateSettingInput {
    pub key: String,
    pub value: String,
    pub category: String,
    pub description: Option<String>,
}

/// 更新设置输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdateSettingInput {
    pub key: String,
    pub value: String,
    pub category: Option<String>,
    pub description: Option<String>,
}

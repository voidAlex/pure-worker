//! 全局快捷键数据模型
//!
//! 定义全局快捷键记录及创建/更新输入结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 全局快捷键记录。
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct GlobalShortcut {
    pub id: String,
    pub action: String,
    pub key_combination: String,
    pub enabled: i32,
    pub description: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建全局快捷键输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateGlobalShortcutInput {
    pub action: String,
    pub key_combination: String,
    pub enabled: Option<i32>,
    pub description: Option<String>,
}

/// 更新全局快捷键输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdateGlobalShortcutInput {
    pub id: String,
    pub action: Option<String>,
    pub key_combination: Option<String>,
    pub enabled: Option<i32>,
    pub description: Option<String>,
}

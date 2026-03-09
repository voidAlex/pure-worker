//! 技能注册数据模型模块
//!
//! 定义技能注册表记录、创建/更新输入以及健康检查结果结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 技能注册表记录。
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
    pub permission_scope: Option<String>,
    pub status: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub skill_type: String,
    pub env_path: Option<String>,
    pub config_json: Option<String>,
    pub updated_at: Option<String>,
    pub health_status: String,
    pub last_health_check: Option<String>,
}

/// 创建技能输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateSkillInput {
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
    pub permission_scope: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub skill_type: String,
    pub config_json: Option<String>,
}

/// 更新技能输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdateSkillInput {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub permission_scope: Option<String>,
    pub config_json: Option<String>,
    pub status: Option<String>,
}

/// 技能健康检查结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillHealthResult {
    pub name: String,
    pub health_status: String,
    pub message: String,
    pub checked_at: String,
}

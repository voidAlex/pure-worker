//! 教师偏好记忆数据模型
//!
//! 定义教师偏好记忆系统的输入输出结构，包括偏好记录、候选记忆、
//! 以及 soul.md / user.md 文件解析相关的数据结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 偏好类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PreferenceType {
    OutputStyle,
    Tone,
    Format,
    Workflow,
    Other,
}

impl std::fmt::Display for PreferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreferenceType::OutputStyle => write!(f, "output_style"),
            PreferenceType::Tone => write!(f, "tone"),
            PreferenceType::Format => write!(f, "format"),
            PreferenceType::Workflow => write!(f, "workflow"),
            PreferenceType::Other => write!(f, "other"),
        }
    }
}

/// 偏好来源枚举
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PreferenceSource {
    Explicit,
    Inferred,
    Imported,
    Default,
}

impl std::fmt::Display for PreferenceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreferenceSource::Explicit => write!(f, "explicit"),
            PreferenceSource::Inferred => write!(f, "inferred"),
            PreferenceSource::Imported => write!(f, "imported"),
            PreferenceSource::Default => write!(f, "default"),
        }
    }
}

/// 教师偏好记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct TeacherPreference {
    pub id: String,
    pub preference_key: String,
    pub preference_value: String,
    pub preference_type: String,
    pub source: String,
    pub confirmed_at: Option<String>,
    pub is_active: i32,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建或更新偏好输入
#[derive(Debug, Deserialize, Type)]
pub struct SetPreferenceInput {
    pub key: String,
    pub value: String,
    pub preference_type: PreferenceType,
    pub source: Option<PreferenceSource>,
}

/// 查询偏好输入
#[derive(Debug, Deserialize, Type)]
pub struct GetPreferenceInput {
    pub key: String,
}

/// 按类型查询偏好输入
#[derive(Debug, Deserialize, Type)]
pub struct ListPreferencesByTypeInput {
    pub preference_type: PreferenceType,
}

/// 候选记忆状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Pending,
    Confirmed,
    Rejected,
}

impl std::fmt::Display for CandidateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidateStatus::Pending => write!(f, "pending"),
            CandidateStatus::Confirmed => write!(f, "confirmed"),
            CandidateStatus::Rejected => write!(f, "rejected"),
        }
    }
}

/// 候选记忆记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct MemoryCandidate {
    pub id: String,
    pub candidate_key: String,
    pub candidate_value: String,
    pub detected_count: i32,
    pub confidence_score: Option<f64>,
    pub pattern_evidence: Option<String>,
    pub status: String,
    pub confirmed_at: Option<String>,
    pub rejected_at: Option<String>,
    pub rejection_reason: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 候选记忆确认输入
#[derive(Debug, Deserialize, Type)]
pub struct ConfirmCandidateInput {
    pub candidate_id: String,
}

/// 候选记忆拒绝输入
#[derive(Debug, Deserialize, Type)]
pub struct RejectCandidateInput {
    pub candidate_id: String,
    pub reason: Option<String>,
}

/// 检测模式输入
#[derive(Debug, Deserialize, Type)]
pub struct DetectPatternInput {
    pub pattern_type: String,
    pub pattern_key: String,
    pub pattern_value: Option<String>,
    pub context: Option<String>,
}

/// 候选记忆筛选输入
#[derive(Debug, Deserialize, Type)]
pub struct ListCandidatesInput {
    pub status: Option<CandidateStatus>,
    pub limit: Option<i32>,
}

/// 系统提示词上下文注入结果
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SystemPromptContext {
    pub soul_md_content: Option<String>,
    pub user_md_content: Option<String>,
    pub active_preferences: Vec<TeacherPreference>,
    pub formatted_context: String,
}

/// Soul.md / User.md 文件解析结果
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SoulMdContent {
    pub version: Option<String>,
    pub description: Option<String>,
    pub sections: Vec<SoulMdSection>,
    pub raw_content: String,
}

/// Soul.md 章节
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SoulMdSection {
    pub title: String,
    pub content: String,
    pub level: i32,
}

/// 重新加载 soul.md 输入
#[derive(Debug, Deserialize, Type)]
pub struct ReloadSoulMdInput {
    pub force_create: Option<bool>,
}

/// Markdown frontmatter 元数据
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SoulMdFrontmatter {
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub last_updated: Option<String>,
    pub tags: Option<Vec<String>>,
}

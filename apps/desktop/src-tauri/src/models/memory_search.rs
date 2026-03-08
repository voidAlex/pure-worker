//! 记忆检索数据模型
//!
//! 定义 Agentic Search 记忆检索相关的输入输出结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 证据项：检索结果的统一数据结构。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EvidenceItem {
    /// 证据内容文本。
    pub content: String,
    /// 来源表（observation_note / parent_communication / semester_comment / file）。
    pub source_table: String,
    /// 来源记录 ID。
    pub source_id: String,
    /// 关联学生 ID。
    pub student_id: String,
    /// 关联班级 ID（可选）。
    pub class_id: Option<String>,
    /// 创建时间（ISO 8601）。
    pub created_at: String,
    /// 相关度评分（0.0 ~ 1.0，规则重排后）。
    pub score: f64,
    /// 证据来源文件路径（仅文件类型证据有值）。
    pub file_path: Option<String>,
    /// 学科标签（可选）。
    pub subject: Option<String>,
}

/// SQL 精确过滤条件输入。
#[derive(Debug, Deserialize, Type)]
pub struct MemorySearchInput {
    /// 搜索关键词。
    pub keyword: Option<String>,
    /// 学生 ID（精确匹配）。
    pub student_id: Option<String>,
    /// 班级 ID（精确匹配）。
    pub class_id: Option<String>,
    /// 时间窗口起始（ISO 8601）。
    pub from_date: Option<String>,
    /// 时间窗口截止（ISO 8601）。
    pub to_date: Option<String>,
    /// 学科过滤（可选）。
    pub subject: Option<String>,
    /// 来源表过滤（可选，如 observation_note）。
    pub source_table: Option<String>,
    /// 返回结果数上限。
    pub top_k: Option<i64>,
    /// 工作区路径（可选，命令层会提供默认值）。
    pub workspace_path: Option<String>,
}

/// FTS 检索结果行（内部使用，从 memory_fts 表查询）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FtsRow {
    /// 来源表。
    pub source_table: String,
    /// 来源记录 ID。
    pub source_id: String,
    /// 学生 ID。
    pub student_id: String,
    /// 班级 ID（FTS 表中为空字符串时代表无值）。
    pub class_id: String,
    /// 检索内容。
    pub content: String,
    /// 创建时间（ISO 8601）。
    pub created_at: String,
    /// FTS 排名值。
    pub rank: f64,
}

/// 统一证据检索结果。
#[derive(Debug, Serialize, Type)]
pub struct SearchEvidenceResult {
    /// 检索到的证据列表（已排序）。
    pub items: Vec<EvidenceItem>,
    /// 检索到的总条数（去重前）。
    pub total_before_dedup: i64,
    /// 实际返回条数。
    pub returned_count: i64,
}

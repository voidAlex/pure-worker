//! 学生长期记忆数据模型。
//!
//! 定义学生长期记忆 Markdown 体系的输入输出结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 从 Markdown 解析出的单条记忆条目。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MemoryEntry {
    /// 条目中的日期标签（可选，格式建议为 YYYY-MM-DD）。
    pub date: Option<String>,
    /// 条目中的学科标签（可选）。
    pub subject: Option<String>,
    /// 条目中的类型标签（可选）。
    pub entry_type: Option<String>,
    /// 条目的正文内容。
    pub content: String,
    /// 条目所属章节名称。
    pub section: String,
    /// 条目来源文件路径。
    pub source_file: String,
}

/// 记忆文件 YAML frontmatter 元数据。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MemoryFileMeta {
    /// 学生 ID。
    pub student_id: String,
    /// 学生姓名（可选）。
    pub student_name: Option<String>,
    /// 班级 ID（可选）。
    pub class_id: Option<String>,
    /// 班主任教师 ID（可选）。
    pub homeroom_teacher_id: Option<String>,
    /// 模板版本号（可选）。
    pub version: Option<String>,
    /// 最近更新时间（可选，ISO 8601）。
    pub last_updated_at: Option<String>,
    /// 文件路径。
    pub file_path: String,
}

/// 读取时间线记忆的输入参数。
#[derive(Debug, Deserialize, Type)]
pub struct ReadMemoryTimelineInput {
    /// 学生 ID。
    pub student_id: String,
    /// 起始日期（可选，YYYY-MM-DD）。
    pub from_date: Option<String>,
    /// 结束日期（可选，YYYY-MM-DD）。
    pub to_date: Option<String>,
    /// 章节过滤列表（可选）。
    pub section_filter: Option<Vec<String>>,
    /// 返回条数上限（可选）。
    pub limit: Option<i32>,
}

/// 按主题检索记忆的输入参数。
#[derive(Debug, Deserialize, Type)]
pub struct ReadMemoryByTopicInput {
    /// 学生 ID。
    pub student_id: String,
    /// 主题关键词。
    pub topic: String,
    /// 学科过滤（可选）。
    pub subject: Option<String>,
    /// 返回条数上限（可选）。
    pub top_k: Option<i32>,
}

/// 读取评语素材的输入参数。
#[derive(Debug, Deserialize, Type)]
pub struct ReadCommentMaterialsInput {
    /// 学生 ID。
    pub student_id: String,
    /// 学期/时间段筛选（可选）。
    pub term: Option<String>,
    /// 学科过滤（可选）。
    pub subject: Option<String>,
}

/// 追加记忆笔记的输入参数。
#[derive(Debug, Deserialize, Type)]
pub struct AppendMemoryNoteInput {
    /// 学生 ID。
    pub student_id: String,
    /// 目标章节名称。
    pub section: String,
    /// 记忆正文内容。
    pub content: String,
}

/// 初始化学生长期记忆目录与模板的输入参数。
#[derive(Debug, Deserialize, Type)]
pub struct InitStudentMemoryInput {
    /// 学生 ID。
    pub student_id: String,
    /// 学生姓名（可选）。
    pub student_name: Option<String>,
    /// 班级 ID（可选）。
    pub class_id: Option<String>,
    /// 班主任教师 ID（可选）。
    pub homeroom_teacher_id: Option<String>,
}

/// 敏感信息检测结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SensitiveInfoResult {
    /// 是否检测到敏感信息。
    pub has_sensitive: bool,
    /// 命中的违规类型列表。
    pub violations: Vec<String>,
}

//! 学期评语数据模型
//!
//! 定义学期评语的结构体及创建/更新输入类型

use serde::{Deserialize, Serialize};
use specta::Type;

/// 学期评语记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct SemesterComment {
    pub id: String,
    pub student_id: String,
    pub task_id: Option<String>,
    pub term: String,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub status: String,
    pub evidence_json: Option<String>,
    pub evidence_count: i32,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建学期评语输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateSemesterCommentInput {
    pub student_id: String,
    pub task_id: Option<String>,
    pub term: String,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub status: Option<String>,
    pub evidence_json: Option<String>,
    pub evidence_count: Option<i32>,
}

/// 更新学期评语输入
#[derive(Debug, Deserialize, Type)]
pub struct UpdateSemesterCommentInput {
    pub id: String,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub status: Option<String>,
    pub evidence_json: Option<String>,
    pub evidence_count: Option<i32>,
}

/// 列表查询输入
#[derive(Debug, Deserialize, Type)]
pub struct ListSemesterCommentsInput {
    pub student_id: Option<String>,
    pub term: Option<String>,
    pub task_id: Option<String>,
}

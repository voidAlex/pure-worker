use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct Classroom {
    pub id: String,
    pub grade: String,
    pub class_name: String,
    pub subject: String,
    pub teacher_id: String,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建班级输入参数。
#[derive(Debug, Deserialize, Type)]
pub struct CreateClassroomInput {
    pub grade: String,
    pub class_name: String,
    pub subject: String,
    /// 教师ID（可选，为空时自动分配默认教师）。
    pub teacher_id: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
pub struct UpdateClassroomInput {
    pub id: String,
    pub grade: Option<String>,
    pub class_name: Option<String>,
    pub subject: Option<String>,
    pub teacher_id: Option<String>,
}

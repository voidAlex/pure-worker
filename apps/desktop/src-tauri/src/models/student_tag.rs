use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct StudentTag {
    pub id: String,
    pub student_id: String,
    pub tag_name: String,
    pub is_deleted: i32,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Type)]
pub struct AddStudentTagInput {
    pub student_id: String,
    pub tag_name: String,
}

/// 更新学生标签输入参数
#[derive(Debug, Deserialize, Type)]
pub struct UpdateStudentTagInput {
    pub id: String,
    pub tag_name: String,
}

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

#[derive(Debug, Deserialize, Type)]
pub struct CreateClassroomInput {
    pub grade: String,
    pub class_name: String,
    pub subject: String,
    pub teacher_id: String,
}

#[derive(Debug, Deserialize, Type)]
pub struct UpdateClassroomInput {
    pub id: String,
    pub grade: Option<String>,
    pub class_name: Option<String>,
    pub subject: Option<String>,
    pub teacher_id: Option<String>,
}

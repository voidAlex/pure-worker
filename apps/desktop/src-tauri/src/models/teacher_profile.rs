use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct TeacherProfile {
    pub id: String,
    pub name: String,
    pub stage: String,
    pub subject: String,
    pub textbook_version: Option<String>,
    pub tone_preset: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

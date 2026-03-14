use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct ObservationNote {
    pub id: String,
    pub student_id: String,
    pub content: String,
    pub source: Option<String>,
    pub created_at: String,
    pub is_deleted: i32,
    pub updated_at: String,
    pub lesson_record_id: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateObservationNoteInput {
    pub student_id: String,
    pub content: String,
    pub source: Option<String>,
    pub lesson_record_id: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
pub struct UpdateObservationNoteInput {
    pub id: String,
    pub content: Option<String>,
    pub source: Option<String>,
    pub lesson_record_id: Option<String>,
}

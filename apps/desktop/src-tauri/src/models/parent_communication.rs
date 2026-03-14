use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct ParentCommunication {
    pub id: String,
    pub student_id: String,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub status: Option<String>,
    pub evidence_json: Option<String>,
    pub created_at: String,
    pub is_deleted: i32,
    pub updated_at: String,
    pub lesson_record_id: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateParentCommunicationInput {
    pub student_id: String,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub status: Option<String>,
    pub evidence_json: Option<String>,
    pub lesson_record_id: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
pub struct UpdateParentCommunicationInput {
    pub id: String,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub status: Option<String>,
    pub evidence_json: Option<String>,
    pub lesson_record_id: Option<String>,
}

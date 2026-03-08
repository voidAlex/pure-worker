use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct ScheduleEvent {
    pub id: String,
    pub class_id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: Option<String>,
    pub linked_file_id: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateScheduleEventInput {
    pub class_id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: Option<String>,
    pub linked_file_id: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
pub struct UpdateScheduleEventInput {
    pub id: String,
    pub title: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub linked_file_id: Option<String>,
}

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct TaskSummary {
    pub id: String,
    pub task_type: String,
    pub status: String,
    pub created_at: String,
}

#[tauri::command]
#[specta::specta]
pub fn list_tasks() -> Result<Vec<TaskSummary>, AppError> {
    Ok(Vec::new())
}

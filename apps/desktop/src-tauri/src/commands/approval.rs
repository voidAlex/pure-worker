use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ApprovalRequest {
    pub id: String,
    pub request_type: String,
    pub action_summary: String,
    pub risk_level: String,
    pub status: String,
}

#[tauri::command]
#[specta::specta]
pub fn list_pending_approvals() -> Result<Vec<ApprovalRequest>, AppError> {
    Ok(Vec::new())
}

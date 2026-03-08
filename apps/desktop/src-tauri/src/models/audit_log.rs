use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct AuditLog {
    pub id: String,
    pub actor: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub risk_level: String,
    pub confirmed_by_user: i32,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateAuditLogInput {
    pub actor: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub risk_level: String,
    pub confirmed_by_user: i32,
}

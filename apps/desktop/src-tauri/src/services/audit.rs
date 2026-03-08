use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;

pub struct AuditService;

impl AuditService {
    pub async fn log(
        pool: &SqlitePool,
        actor: &str,
        action: &str,
        target_type: &str,
        target_id: Option<&str>,
        risk_level: &str,
        confirmed_by_user: bool,
    ) -> Result<(), AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO audit_log (id, actor, action, target_type, target_id, risk_level, confirmed_by_user, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(actor)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(risk_level)
        .bind(confirmed_by_user as i32)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }
}

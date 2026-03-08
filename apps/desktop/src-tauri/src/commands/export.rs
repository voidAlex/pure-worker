use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub db_connected: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn health_check(pool: State<'_, SqlitePool>) -> Result<HealthCheckResponse, AppError> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&*pool)
        .await?;

    Ok(HealthCheckResponse {
        status: String::from("ok"),
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_connected: true,
    })
}

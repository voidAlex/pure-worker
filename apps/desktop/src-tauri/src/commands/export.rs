use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub db_connected: bool,
}

#[tauri::command]
#[specta::specta]
pub fn health_check() -> Result<HealthCheckResponse, AppError> {
    Ok(HealthCheckResponse {
        status: String::from("ok"),
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_connected: false,
    })
}

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct AppSettings {
    pub workspace_path: String,
    pub ai_provider: String,
    pub language: String,
}

#[tauri::command]
#[specta::specta]
pub fn get_app_settings() -> Result<AppSettings, AppError> {
    Ok(AppSettings {
        workspace_path: String::from("./workspace"),
        ai_provider: String::from("未配置"),
        language: String::from("zh-CN"),
    })
}

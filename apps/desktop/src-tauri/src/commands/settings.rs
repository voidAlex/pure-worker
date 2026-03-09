//! 应用设置 IPC 命令模块
//!
//! 提供应用设置的查询与更新命令，数据来源于数据库持久化层。

use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::app_settings::AppSetting;
use crate::services;

/// 获取全部应用设置。
#[tauri::command]
#[specta::specta]
pub async fn get_app_settings(pool: State<'_, SqlitePool>) -> Result<Vec<AppSetting>, AppError> {
    services::app_settings::AppSettingsService::list_settings(&pool).await
}

/// 根据 key 获取单个设置。
#[tauri::command]
#[specta::specta]
pub async fn get_setting(pool: State<'_, SqlitePool>, key: String) -> Result<AppSetting, AppError> {
    services::app_settings::AppSettingsService::get_setting(&pool, &key).await
}

/// 更新（插入或覆盖）单个设置。
#[tauri::command]
#[specta::specta]
pub async fn update_setting(
    pool: State<'_, SqlitePool>,
    key: String,
    value: String,
    category: String,
    description: Option<String>,
) -> Result<AppSetting, AppError> {
    services::app_settings::AppSettingsService::upsert_setting(
        &pool,
        &key,
        &value,
        &category,
        description.as_deref(),
    )
    .await
}

/// 根据分类查询设置。
#[tauri::command]
#[specta::specta]
pub async fn get_settings_by_category(
    pool: State<'_, SqlitePool>,
    category: String,
) -> Result<Vec<AppSetting>, AppError> {
    services::app_settings::AppSettingsService::get_settings_by_category(&pool, &category).await
}

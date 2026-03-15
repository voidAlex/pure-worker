//! 应用设置 IPC 命令模块
//!
//! 提供应用设置的查询与更新命令，数据来源于数据库持久化层。

use std::path::Path;

use sqlx::SqlitePool;
use tauri::State;
use tauri_plugin_shell::ShellExt;

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

/// 在系统文件管理器中打开 Skills 目录。
///
/// 如果目录不存在，会自动创建。
#[tauri::command]
#[specta::specta]
pub async fn open_skills_directory(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<String, AppError> {
    // 获取工作区路径
    let setting = services::app_settings::AppSettingsService::get_setting(&pool, "workspace_path")
        .await
        .map_err(|e| AppError::Config(format!("获取工作区路径失败：{}", e)))?;

    let workspace_path_str = setting
        .value
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_string();
    let workspace_path = Path::new(&workspace_path_str);

    // 构建并验证 skills 目录路径
    let (_canonical_workspace, skills_dir) =
        crate::services::path_whitelist::PathWhitelistService::ensure_safe_skills_dir(
            workspace_path,
        )
        .map_err(|e| AppError::InvalidInput(format!("Skills 目录验证失败：{}", e)))?;

    let skills_dir_str = skills_dir.to_string_lossy().to_string();

    // 使用系统默认程序打开目录
    #[allow(deprecated)]
    app.shell()
        .open(&skills_dir_str, None)
        .map_err(|e| AppError::ExternalService(format!("打开目录失败：{}", e)))?;

    Ok(skills_dir_str)
}

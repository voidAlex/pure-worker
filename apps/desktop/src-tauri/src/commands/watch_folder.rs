//! 监控文件夹 IPC 命令模块
//!
//! 提供监控文件夹的查询与增删改命令。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::watch_folder::{CreateWatchFolderInput, UpdateWatchFolderInput, WatchFolder};
use crate::services;

/// 删除监控文件夹输入。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteWatchFolderInput {
    pub id: String,
}

/// 删除监控文件夹响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteWatchFolderResponse {
    pub success: bool,
}

/// 获取全部监控文件夹。
#[tauri::command]
#[specta::specta]
pub async fn list_watch_folders(pool: State<'_, SqlitePool>) -> Result<Vec<WatchFolder>, AppError> {
    services::watch_folder::WatchFolderService::list_folders(&pool).await
}

/// 根据 ID 获取监控文件夹。
#[tauri::command]
#[specta::specta]
pub async fn get_watch_folder(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<WatchFolder, AppError> {
    services::watch_folder::WatchFolderService::get_folder(&pool, &id).await
}

/// 创建监控文件夹。
#[tauri::command]
#[specta::specta]
pub async fn create_watch_folder(
    pool: State<'_, SqlitePool>,
    input: CreateWatchFolderInput,
) -> Result<WatchFolder, AppError> {
    services::watch_folder::WatchFolderService::create_folder(&pool, input).await
}

/// 更新监控文件夹。
#[tauri::command]
#[specta::specta]
pub async fn update_watch_folder(
    pool: State<'_, SqlitePool>,
    input: UpdateWatchFolderInput,
) -> Result<WatchFolder, AppError> {
    services::watch_folder::WatchFolderService::update_folder(&pool, input).await
}

/// 删除监控文件夹。
#[tauri::command]
#[specta::specta]
pub async fn delete_watch_folder(
    pool: State<'_, SqlitePool>,
    input: DeleteWatchFolderInput,
) -> Result<DeleteWatchFolderResponse, AppError> {
    services::watch_folder::WatchFolderService::delete_folder(&pool, &input.id).await?;

    Ok(DeleteWatchFolderResponse { success: true })
}

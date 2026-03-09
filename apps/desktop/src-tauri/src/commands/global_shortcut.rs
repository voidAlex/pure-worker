//! 全局快捷键 IPC 命令模块
//!
//! 提供全局快捷键的查询与增删改命令。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::global_shortcut::{
    CreateGlobalShortcutInput, GlobalShortcut, UpdateGlobalShortcutInput,
};
use crate::services;

/// 删除全局快捷键输入。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteGlobalShortcutInput {
    pub id: String,
}

/// 删除全局快捷键响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteGlobalShortcutResponse {
    pub success: bool,
}

/// 获取全部全局快捷键。
#[tauri::command]
#[specta::specta]
pub async fn list_global_shortcuts(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<GlobalShortcut>, AppError> {
    services::global_shortcut::GlobalShortcutService::list_shortcuts(&pool).await
}

/// 根据 ID 获取全局快捷键。
#[tauri::command]
#[specta::specta]
pub async fn get_global_shortcut(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<GlobalShortcut, AppError> {
    services::global_shortcut::GlobalShortcutService::get_shortcut(&pool, &id).await
}

/// 创建全局快捷键。
#[tauri::command]
#[specta::specta]
pub async fn create_global_shortcut(
    pool: State<'_, SqlitePool>,
    input: CreateGlobalShortcutInput,
) -> Result<GlobalShortcut, AppError> {
    services::global_shortcut::GlobalShortcutService::create_shortcut(&pool, input).await
}

/// 更新全局快捷键。
#[tauri::command]
#[specta::specta]
pub async fn update_global_shortcut(
    pool: State<'_, SqlitePool>,
    input: UpdateGlobalShortcutInput,
) -> Result<GlobalShortcut, AppError> {
    services::global_shortcut::GlobalShortcutService::update_shortcut(&pool, input).await
}

/// 删除全局快捷键。
#[tauri::command]
#[specta::specta]
pub async fn delete_global_shortcut(
    pool: State<'_, SqlitePool>,
    input: DeleteGlobalShortcutInput,
) -> Result<DeleteGlobalShortcutResponse, AppError> {
    services::global_shortcut::GlobalShortcutService::delete_shortcut(&pool, &input.id).await?;

    Ok(DeleteGlobalShortcutResponse { success: true })
}

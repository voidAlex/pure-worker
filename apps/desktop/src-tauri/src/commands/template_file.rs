//! 校本模板文件命令模块
//!
//! 提供校本模板文件的 IPC 命令接口。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::template_file::{
    CreateTemplateFileInput, ListTemplateFilesInput, TemplateFile, UpdateTemplateFileInput,
};
use crate::services;

/// 删除模板文件输入参数
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteTemplateFileInput {
    pub id: String,
}

/// 删除模板文件响应
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteTemplateFileResponse {
    pub success: bool,
}

/// 列表查询模板文件
#[tauri::command]
#[specta::specta]
pub async fn list_template_files(
    pool: State<'_, SqlitePool>,
    input: ListTemplateFilesInput,
) -> Result<Vec<TemplateFile>, AppError> {
    services::template_file::TemplateFileService::list(&pool, input).await
}

/// 根据 ID 获取模板文件
#[tauri::command]
#[specta::specta]
pub async fn get_template_file(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<TemplateFile, AppError> {
    services::template_file::TemplateFileService::get_by_id(&pool, &id).await
}

/// 创建模板文件
#[tauri::command]
#[specta::specta]
pub async fn create_template_file(
    pool: State<'_, SqlitePool>,
    input: CreateTemplateFileInput,
) -> Result<TemplateFile, AppError> {
    services::template_file::TemplateFileService::create(&pool, input).await
}

/// 更新模板文件
#[tauri::command]
#[specta::specta]
pub async fn update_template_file(
    pool: State<'_, SqlitePool>,
    input: UpdateTemplateFileInput,
) -> Result<TemplateFile, AppError> {
    services::template_file::TemplateFileService::update(&pool, input).await
}

/// 删除模板文件
#[tauri::command]
#[specta::specta]
pub async fn delete_template_file(
    pool: State<'_, SqlitePool>,
    input: DeleteTemplateFileInput,
) -> Result<DeleteTemplateFileResponse, AppError> {
    services::template_file::TemplateFileService::delete(&pool, &input.id).await?;
    Ok(DeleteTemplateFileResponse { success: true })
}

//! 课表文件命令模块
//!
//! 提供课表事件关联文件的Tauri命令接口

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::schedule_file::{CreateScheduleFileInput, ScheduleFile};
use crate::services;

/// 获取班级文件列表输入参数
#[derive(Debug, Deserialize, Type)]
pub struct ListScheduleFilesInput {
    pub class_id: String,
}

/// 删除文件输入参数
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteScheduleFileInput {
    pub id: String,
}

/// 删除文件响应
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteScheduleFileResponse {
    pub success: bool,
}

/// 获取班级的所有文件列表
#[tauri::command]
#[specta::specta]
pub async fn list_schedule_files(
    pool: State<'_, SqlitePool>,
    input: ListScheduleFilesInput,
) -> Result<Vec<ScheduleFile>, AppError> {
    services::schedule_file::ScheduleFileService::list_by_class(&pool, &input.class_id).await
}

/// 注册新文件
#[tauri::command]
#[specta::specta]
pub async fn create_schedule_file(
    pool: State<'_, SqlitePool>,
    input: CreateScheduleFileInput,
) -> Result<ScheduleFile, AppError> {
    services::schedule_file::ScheduleFileService::register(&pool, input).await
}

/// 删除文件
#[tauri::command]
#[specta::specta]
pub async fn delete_schedule_file(
    pool: State<'_, SqlitePool>,
    input: DeleteScheduleFileInput,
) -> Result<DeleteScheduleFileResponse, AppError> {
    services::schedule_file::ScheduleFileService::delete(&pool, &input.id).await?;
    Ok(DeleteScheduleFileResponse { success: true })
}

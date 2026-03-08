//! 学期评语命令模块
//!
//! 提供学期评语相关 Tauri IPC 命令接口

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::semester_comment::{
    CreateSemesterCommentInput, ListSemesterCommentsInput, SemesterComment,
    UpdateSemesterCommentInput,
};
use crate::services;

/// 删除学期评语输入
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteSemesterCommentInput {
    pub id: String,
}

/// 删除学期评语响应
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteSemesterCommentResponse {
    pub success: bool,
}

/// 批量采纳输入
#[derive(Debug, Deserialize, Type)]
pub struct BatchAdoptInput {
    pub task_id: String,
}

/// 列出学期评语
#[tauri::command]
#[specta::specta]
pub async fn list_semester_comments(
    pool: State<'_, SqlitePool>,
    input: ListSemesterCommentsInput,
) -> Result<Vec<SemesterComment>, AppError> {
    services::semester_comment::SemesterCommentService::list(&pool, input).await
}

/// 创建学期评语
#[tauri::command]
#[specta::specta]
pub async fn create_semester_comment(
    pool: State<'_, SqlitePool>,
    input: CreateSemesterCommentInput,
) -> Result<SemesterComment, AppError> {
    services::semester_comment::SemesterCommentService::create(&pool, input).await
}

/// 更新学期评语
#[tauri::command]
#[specta::specta]
pub async fn update_semester_comment(
    pool: State<'_, SqlitePool>,
    input: UpdateSemesterCommentInput,
) -> Result<SemesterComment, AppError> {
    services::semester_comment::SemesterCommentService::update(&pool, input).await
}

/// 删除学期评语
#[tauri::command]
#[specta::specta]
pub async fn delete_semester_comment(
    pool: State<'_, SqlitePool>,
    input: DeleteSemesterCommentInput,
) -> Result<DeleteSemesterCommentResponse, AppError> {
    services::semester_comment::SemesterCommentService::delete(&pool, &input.id).await?;
    Ok(DeleteSemesterCommentResponse { success: true })
}

/// 按任务批量采纳学期评语
#[tauri::command]
#[specta::specta]
pub async fn batch_adopt_semester_comments(
    pool: State<'_, SqlitePool>,
    input: BatchAdoptInput,
) -> Result<Vec<SemesterComment>, AppError> {
    services::semester_comment::SemesterCommentService::batch_adopt(&pool, &input.task_id).await
}

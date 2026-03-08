use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::student_tag::{AddStudentTagInput, StudentTag, UpdateStudentTagInput};
use crate::services;

#[derive(Debug, Deserialize, Type)]
pub struct ListStudentTagsInput {
    pub student_id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct RemoveStudentTagInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct RemoveStudentTagResponse {
    pub success: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn list_student_tags(
    pool: State<'_, SqlitePool>,
    input: ListStudentTagsInput,
) -> Result<Vec<StudentTag>, AppError> {
    services::student_tag::StudentTagService::list_by_student(&pool, &input.student_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn add_student_tag(
    pool: State<'_, SqlitePool>,
    input: AddStudentTagInput,
) -> Result<StudentTag, AppError> {
    services::student_tag::StudentTagService::add(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_student_tag(
    pool: State<'_, SqlitePool>,
    input: RemoveStudentTagInput,
) -> Result<RemoveStudentTagResponse, AppError> {
    services::student_tag::StudentTagService::remove(&pool, &input.id).await?;
    Ok(RemoveStudentTagResponse { success: true })
}

/// 更新学生标签
#[tauri::command]
#[specta::specta]
pub async fn update_student_tag(
    pool: State<'_, SqlitePool>,
    input: UpdateStudentTagInput,
) -> Result<StudentTag, AppError> {
    services::student_tag::StudentTagService::update(&pool, input).await
}

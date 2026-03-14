use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::lesson_record::{
    CreateLessonRecordInput, LessonRecord, LessonSummary, ListLessonRecordsInput,
    UpdateLessonRecordInput,
};
use crate::services;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteLessonRecordInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteLessonRecordResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct GetLessonSummaryInput {
    pub lesson_record_id: String,
}

#[tauri::command]
#[specta::specta]
pub async fn create_lesson_record(
    pool: State<'_, SqlitePool>,
    input: CreateLessonRecordInput,
) -> Result<LessonRecord, AppError> {
    services::lesson_record::LessonRecordService::create_lesson_record(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_lesson_record(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<LessonRecord, AppError> {
    services::lesson_record::LessonRecordService::get_lesson_record(&pool, &id).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_lesson_record(
    pool: State<'_, SqlitePool>,
    input: UpdateLessonRecordInput,
) -> Result<LessonRecord, AppError> {
    services::lesson_record::LessonRecordService::update_lesson_record(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_lesson_record(
    pool: State<'_, SqlitePool>,
    input: DeleteLessonRecordInput,
) -> Result<DeleteLessonRecordResponse, AppError> {
    services::lesson_record::LessonRecordService::delete_lesson_record(&pool, &input.id).await?;
    Ok(DeleteLessonRecordResponse { success: true })
}

#[tauri::command]
#[specta::specta]
pub async fn list_lesson_records(
    pool: State<'_, SqlitePool>,
    input: ListLessonRecordsInput,
) -> Result<Vec<LessonRecord>, AppError> {
    services::lesson_record::LessonRecordService::list_lesson_records(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_lesson_summary(
    pool: State<'_, SqlitePool>,
    input: GetLessonSummaryInput,
) -> Result<LessonSummary, AppError> {
    services::lesson_record::LessonRecordService::get_lesson_summary(&pool, &input.lesson_record_id)
        .await
}

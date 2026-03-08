use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::score_record::{CreateScoreRecordInput, ScoreRecord, UpdateScoreRecordInput};
use crate::services;

#[derive(Debug, Deserialize, Type)]
pub struct ListStudentScoresInput {
    pub student_id: String,
    pub subject: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteScoreRecordInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteScoreRecordResponse {
    pub success: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn list_student_scores(
    pool: State<'_, SqlitePool>,
    input: ListStudentScoresInput,
) -> Result<Vec<ScoreRecord>, AppError> {
    services::score_record::ScoreRecordService::list_student_scores(
        &pool,
        &input.student_id,
        input.subject.as_deref(),
        input.from_date.as_deref(),
        input.to_date.as_deref(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_score_record(
    pool: State<'_, SqlitePool>,
    input: CreateScoreRecordInput,
) -> Result<ScoreRecord, AppError> {
    services::score_record::ScoreRecordService::create(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_score_record(
    pool: State<'_, SqlitePool>,
    input: UpdateScoreRecordInput,
) -> Result<ScoreRecord, AppError> {
    services::score_record::ScoreRecordService::update(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_score_record(
    pool: State<'_, SqlitePool>,
    input: DeleteScoreRecordInput,
) -> Result<DeleteScoreRecordResponse, AppError> {
    services::score_record::ScoreRecordService::delete(&pool, &input.id).await?;
    Ok(DeleteScoreRecordResponse { success: true })
}

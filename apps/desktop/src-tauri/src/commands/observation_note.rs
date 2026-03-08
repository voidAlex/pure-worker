use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::observation_note::{
    CreateObservationNoteInput, ObservationNote, UpdateObservationNoteInput,
};
use crate::services;

#[derive(Debug, Deserialize, Type)]
pub struct ListStudentObservationsInput {
    pub student_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteObservationNoteInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteObservationNoteResponse {
    pub success: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn list_student_observations(
    pool: State<'_, SqlitePool>,
    input: ListStudentObservationsInput,
) -> Result<Vec<ObservationNote>, AppError> {
    services::observation_note::ObservationNoteService::list_student_observations(
        &pool,
        &input.student_id,
        input.limit,
        input.offset,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_observation_note(
    pool: State<'_, SqlitePool>,
    input: CreateObservationNoteInput,
) -> Result<ObservationNote, AppError> {
    services::observation_note::ObservationNoteService::create(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_observation_note(
    pool: State<'_, SqlitePool>,
    input: UpdateObservationNoteInput,
) -> Result<ObservationNote, AppError> {
    services::observation_note::ObservationNoteService::update(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_observation_note(
    pool: State<'_, SqlitePool>,
    input: DeleteObservationNoteInput,
) -> Result<DeleteObservationNoteResponse, AppError> {
    services::observation_note::ObservationNoteService::delete(&pool, &input.id).await?;
    Ok(DeleteObservationNoteResponse { success: true })
}

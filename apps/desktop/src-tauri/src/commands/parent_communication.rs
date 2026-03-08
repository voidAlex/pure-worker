use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::parent_communication::{
    CreateParentCommunicationInput, ParentCommunication, UpdateParentCommunicationInput,
};
use crate::services;

#[derive(Debug, Deserialize, Type)]
pub struct ListParentCommunicationsInput {
    pub student_id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteParentCommunicationInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteParentCommunicationResponse {
    pub success: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn list_parent_communications(
    pool: State<'_, SqlitePool>,
    input: ListParentCommunicationsInput,
) -> Result<Vec<ParentCommunication>, AppError> {
    services::parent_communication::ParentCommunicationService::list_by_student(
        &pool,
        &input.student_id,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_parent_communication(
    pool: State<'_, SqlitePool>,
    input: CreateParentCommunicationInput,
) -> Result<ParentCommunication, AppError> {
    services::parent_communication::ParentCommunicationService::create(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_parent_communication(
    pool: State<'_, SqlitePool>,
    input: UpdateParentCommunicationInput,
) -> Result<ParentCommunication, AppError> {
    services::parent_communication::ParentCommunicationService::update(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_parent_communication(
    pool: State<'_, SqlitePool>,
    input: DeleteParentCommunicationInput,
) -> Result<DeleteParentCommunicationResponse, AppError> {
    services::parent_communication::ParentCommunicationService::delete(&pool, &input.id).await?;
    Ok(DeleteParentCommunicationResponse { success: true })
}

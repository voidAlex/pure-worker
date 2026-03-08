use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::classroom::{Classroom, CreateClassroomInput, UpdateClassroomInput};
use crate::services;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteClassroomInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteClassroomResponse {
    pub success: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn list_classrooms(pool: State<'_, SqlitePool>) -> Result<Vec<Classroom>, AppError> {
    services::classroom::ClassroomService::list(&pool).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_classroom(pool: State<'_, SqlitePool>, id: String) -> Result<Classroom, AppError> {
    services::classroom::ClassroomService::get_by_id(&pool, &id).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_classroom(
    pool: State<'_, SqlitePool>,
    input: CreateClassroomInput,
) -> Result<Classroom, AppError> {
    services::classroom::ClassroomService::create(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_classroom(
    pool: State<'_, SqlitePool>,
    input: UpdateClassroomInput,
) -> Result<Classroom, AppError> {
    services::classroom::ClassroomService::update(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_classroom(
    pool: State<'_, SqlitePool>,
    input: DeleteClassroomInput,
) -> Result<DeleteClassroomResponse, AppError> {
    services::classroom::ClassroomService::delete(&pool, &input.id).await?;

    Ok(DeleteClassroomResponse { success: true })
}

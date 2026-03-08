use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::student::{CreateStudentInput, Student, StudentProfile360, UpdateStudentInput};
use crate::services;

#[derive(Debug, Deserialize, Type)]
pub struct ListStudentsInput {
    pub class_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteStudentInput {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteStudentResponse {
    pub success: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn list_students(
    pool: State<'_, SqlitePool>,
    input: ListStudentsInput,
) -> Result<Vec<Student>, AppError> {
    services::student::StudentService::list(&pool, input.class_id.as_deref()).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_student(pool: State<'_, SqlitePool>, id: String) -> Result<Student, AppError> {
    services::student::StudentService::get_by_id(&pool, &id).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_student(
    pool: State<'_, SqlitePool>,
    input: CreateStudentInput,
) -> Result<Student, AppError> {
    services::student::StudentService::create(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn update_student(
    pool: State<'_, SqlitePool>,
    input: UpdateStudentInput,
) -> Result<Student, AppError> {
    services::student::StudentService::update(&pool, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_student(
    pool: State<'_, SqlitePool>,
    input: DeleteStudentInput,
) -> Result<DeleteStudentResponse, AppError> {
    services::student::StudentService::delete(&pool, &input.id).await?;
    Ok(DeleteStudentResponse { success: true })
}

/// 获取学生 360 度全景视图
#[tauri::command]
#[specta::specta]
pub async fn get_student_profile_360(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<StudentProfile360, AppError> {
    services::student::StudentService::get_profile_360(&pool, &id).await
}

use tauri::State;

use crate::error::AppError;
use crate::models::teacher_profile::{CreateTeacherProfileInput, TeacherProfile};
use crate::services::teacher_profile::TeacherProfileService;
use sqlx::SqlitePool;

/// 获取教师档案
#[tauri::command]
#[specta::specta]
pub async fn get_teacher_profile(pool: State<'_, SqlitePool>) -> Result<TeacherProfile, AppError> {
    TeacherProfileService::get_or_create_default(&pool).await
}

/// 创建或更新教师档案
#[tauri::command]
#[specta::specta]
pub async fn create_teacher_profile(
    pool: State<'_, SqlitePool>,
    input: CreateTeacherProfileInput,
) -> Result<TeacherProfile, AppError> {
    TeacherProfileService::create(&pool, input).await
}

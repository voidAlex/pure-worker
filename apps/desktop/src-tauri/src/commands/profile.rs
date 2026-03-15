use tauri::State;

use crate::error::AppError;
use crate::models::teacher_profile::TeacherProfile;
use crate::services::teacher_profile::TeacherProfileService;
use sqlx::SqlitePool;

#[tauri::command]
#[specta::specta]
pub async fn get_teacher_profile(pool: State<'_, SqlitePool>) -> Result<TeacherProfile, AppError> {
    TeacherProfileService::get_or_create_default(&pool).await
}

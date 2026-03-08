use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::student_import::{ImportStudentsInput, ImportStudentsResult};
use crate::services;

#[tauri::command]
#[specta::specta]
pub async fn import_students(
    pool: State<'_, SqlitePool>,
    input: ImportStudentsInput,
) -> Result<ImportStudentsResult, AppError> {
    services::student_import::StudentImportService::import(&pool, input).await
}

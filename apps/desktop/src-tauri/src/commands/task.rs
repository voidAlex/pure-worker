use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::async_task::{AsyncTask, RecoverTaskInput, RecoverableTask};
use crate::services::async_task::AsyncTaskService;

/// 查询任务列表（可选按状态过滤）。
#[tauri::command]
#[specta::specta]
pub async fn list_tasks(
    pool: tauri::State<'_, SqlitePool>,
    status: Option<String>,
) -> Result<Vec<AsyncTask>, AppError> {
    AsyncTaskService::list_tasks(pool.inner(), status.as_deref()).await
}

/// 获取单个任务详情。
#[tauri::command]
#[specta::specta]
pub async fn get_task(
    pool: tauri::State<'_, SqlitePool>,
    task_id: String,
) -> Result<AsyncTask, AppError> {
    AsyncTaskService::get_by_id(pool.inner(), &task_id).await
}

/// 恢复或终止可恢复任务（P-005）。
#[tauri::command]
#[specta::specta]
pub async fn recover_task(
    pool: tauri::State<'_, SqlitePool>,
    input: RecoverTaskInput,
) -> Result<AsyncTask, AppError> {
    if input.resume {
        AsyncTaskService::resume_task(pool.inner(), &input.task_id).await
    } else {
        AsyncTaskService::cancel_recovering_task(pool.inner(), &input.task_id).await
    }
}

/// 查询可恢复任务列表（启动时调用，P-003）。
#[tauri::command]
#[specta::specta]
pub async fn list_recoverable_tasks(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<RecoverableTask>, AppError> {
    AsyncTaskService::find_recoverable_tasks(pool.inner()).await
}

/// 续租任务租约（P-006）。
#[tauri::command]
#[specta::specta]
pub async fn renew_task_lease(
    pool: tauri::State<'_, SqlitePool>,
    task_id: String,
    minutes: i64,
) -> Result<(), AppError> {
    AsyncTaskService::renew_lease(pool.inner(), &task_id, minutes).await
}

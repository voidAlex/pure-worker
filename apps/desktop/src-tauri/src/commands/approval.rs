//! 审批请求 IPC 命令模块。
//!
//! 提供审批请求查询、处理与过期清理命令。

use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::approval_request::{ApprovalRequest, ResolveApprovalInput};
use crate::services::approval::ApprovalService;

/// 查询待处理审批列表。
#[tauri::command]
#[specta::specta]
pub async fn list_pending_approvals(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<ApprovalRequest>, AppError> {
    ApprovalService::list_pending(pool.inner()).await
}

/// 解决审批请求（批准或拒绝）。
#[tauri::command]
#[specta::specta]
pub async fn resolve_approval(
    pool: tauri::State<'_, SqlitePool>,
    input: ResolveApprovalInput,
) -> Result<ApprovalRequest, AppError> {
    ApprovalService::resolve(pool.inner(), input).await
}

/// 清理过期审批请求。
#[tauri::command]
#[specta::specta]
pub async fn cleanup_expired_approvals(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<i32, AppError> {
    ApprovalService::cleanup_expired(pool.inner()).await
}

/// 查询指定任务的审批记录。
#[tauri::command]
#[specta::specta]
pub async fn list_task_approvals(
    pool: tauri::State<'_, SqlitePool>,
    task_id: String,
) -> Result<Vec<ApprovalRequest>, AppError> {
    ApprovalService::list_by_task(pool.inner(), &task_id).await
}

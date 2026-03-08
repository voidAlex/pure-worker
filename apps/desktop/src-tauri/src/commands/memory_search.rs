//! 记忆检索 IPC 命令模块
//!
//! 暴露 Agentic Search 的统一检索命令。

use std::path::PathBuf;

use sqlx::SqlitePool;
use tauri::{Manager, State};

use crate::error::AppError;
use crate::models::memory_search::{MemorySearchInput, SearchEvidenceResult};
use crate::services::memory_search::MemorySearchService;

/// 统一证据检索 IPC 命令。
#[tauri::command]
#[specta::specta]
pub async fn search_evidence(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: MemorySearchInput,
) -> Result<SearchEvidenceResult, AppError> {
    let workspace_path = resolve_workspace_path(&app_handle, &input)?;
    MemorySearchService::search_evidence(&pool, &workspace_path, input).await
}

/// 解析工作区路径，优先使用输入参数，其次回退到应用数据目录。
fn resolve_workspace_path(
    app_handle: &tauri::AppHandle,
    input: &MemorySearchInput,
) -> Result<PathBuf, AppError> {
    if let Some(workspace_path) = input.workspace_path.as_deref() {
        let path = workspace_path.trim();
        if path.is_empty() {
            return Err(AppError::InvalidInput(String::from(
                "workspace_path 不能为空字符串",
            )));
        }
        return Ok(PathBuf::from(path));
    }

    let app_data_dir = app_handle.path().app_data_dir().map_err(|error| {
        AppError::Config(format!(
            "获取应用数据目录失败，无法推导 workspace_path：{}",
            error
        ))
    })?;

    Ok(app_data_dir.join("workspace"))
}

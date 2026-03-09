//! 存储生命周期 IPC 命令模块。
//!
//! 暴露工作区统计、导出、归档、擦除命令。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::storage_lifecycle::StorageStats;
use crate::services;

/// 导出工作区输入。
#[derive(Debug, Deserialize, Type)]
pub struct ExportWorkspaceInput {
    /// 导出 ZIP 文件完整路径。
    pub output_path: String,
    /// 是否已完成用户确认（高危确认开启时必填 true）。
    pub approved: bool,
}

/// 导出工作区响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ExportWorkspaceResponse {
    /// 导出输出路径。
    pub output_path: String,
}

/// 归档工作区输入。
#[derive(Debug, Deserialize, Type)]
pub struct ArchiveWorkspaceInput {
    /// 归档名称，空值时自动生成。
    pub archive_name: Option<String>,
}

/// 归档工作区响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ArchiveWorkspaceResponse {
    /// 归档目录路径。
    pub archive_path: String,
}

/// 擦除工作区输入。
#[derive(Debug, Deserialize, Type)]
pub struct EraseWorkspaceInput {
    /// 是否已完成用户确认。
    pub approved: bool,
}

/// 擦除工作区响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct EraseWorkspaceResponse {
    /// 执行是否成功。
    pub success: bool,
}

/// 获取工作区存储统计信息。
#[tauri::command]
#[specta::specta]
pub async fn get_storage_stats(pool: State<'_, SqlitePool>) -> Result<StorageStats, AppError> {
    services::storage_lifecycle::StorageLifecycleService::get_storage_stats(&pool).await
}

/// 导出工作区数据。
#[tauri::command]
#[specta::specta]
pub async fn export_workspace(
    pool: State<'_, SqlitePool>,
    input: ExportWorkspaceInput,
) -> Result<ExportWorkspaceResponse, AppError> {
    let required = services::high_risk_gate::HighRiskGateService::requires_confirmation(
        &pool,
        "export_workspace",
    )
    .await?;
    if required && !input.approved {
        return Err(AppError::PermissionDenied(String::from(
            "导出工作区属于高危操作，必须先完成用户确认",
        )));
    }

    let output_path = services::storage_lifecycle::StorageLifecycleService::export_workspace(
        &pool,
        &input.output_path,
    )
    .await?;
    Ok(ExportWorkspaceResponse { output_path })
}

/// 归档工作区数据。
#[tauri::command]
#[specta::specta]
pub async fn archive_workspace(
    pool: State<'_, SqlitePool>,
    input: ArchiveWorkspaceInput,
) -> Result<ArchiveWorkspaceResponse, AppError> {
    let archive_name = input
        .archive_name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(services::storage_lifecycle::StorageLifecycleService::default_archive_name);

    let archive_path = services::storage_lifecycle::StorageLifecycleService::archive_workspace(
        &pool,
        &archive_name,
    )
    .await?;

    Ok(ArchiveWorkspaceResponse { archive_path })
}

/// 擦除工作区数据（高危）。
#[tauri::command]
#[specta::specta]
pub async fn erase_workspace(
    pool: State<'_, SqlitePool>,
    input: EraseWorkspaceInput,
) -> Result<EraseWorkspaceResponse, AppError> {
    let required = services::high_risk_gate::HighRiskGateService::requires_confirmation(
        &pool,
        "erase_workspace",
    )
    .await?;
    if required && !input.approved {
        return Err(AppError::PermissionDenied(String::from(
            "擦除工作区属于高危操作，必须先完成用户确认",
        )));
    }

    services::storage_lifecycle::StorageLifecycleService::erase_workspace(&pool).await?;
    Ok(EraseWorkspaceResponse { success: true })
}

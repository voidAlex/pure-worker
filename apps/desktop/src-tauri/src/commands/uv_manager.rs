//! uv 管理 IPC 命令模块
//!
//! 暴露 uv 健康检查、环境创建、安装与修复命令。

use serde::Deserialize;
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::services::audit::AuditService;
use crate::services::uv_manager::{UvHealthResult, UvInstallResult, UvManager};

/// 创建技能环境输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateSkillEnvInput {
    pub skill_name: String,
    pub python_version: Option<String>,
}

/// 检查 uv 健康状态。
#[tauri::command]
#[specta::specta]
pub async fn check_uv_health() -> Result<UvHealthResult, AppError> {
    UvManager::check_uv_health().await
}

/// 创建技能环境。
#[tauri::command]
#[specta::specta]
pub async fn create_skill_env(input: CreateSkillEnvInput) -> Result<String, AppError> {
    UvManager::create_skill_env(&input.skill_name, input.python_version.as_deref()).await
}

/// 安装 uv。
#[tauri::command]
#[specta::specta]
pub async fn install_uv(pool: State<'_, SqlitePool>) -> Result<UvInstallResult, AppError> {
    let result = UvManager::install_uv().await?;
    AuditService::log(
        &pool,
        "system",
        "install_uv",
        "uv_manager",
        None,
        "high",
        true,
    )
    .await?;
    Ok(result)
}

/// 修复 uv。
#[tauri::command]
#[specta::specta]
pub async fn repair_uv(pool: State<'_, SqlitePool>) -> Result<UvInstallResult, AppError> {
    let result = UvManager::repair_uv().await?;
    AuditService::log(
        &pool,
        "system",
        "repair_uv",
        "uv_manager",
        None,
        "high",
        true,
    )
    .await?;
    Ok(result)
}

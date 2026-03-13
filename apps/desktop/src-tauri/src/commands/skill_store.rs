//! 技能商店 IPC 命令模块
//!
//! 暴露技能商店的列表、安装、卸载能力给前端调用。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use std::path::Path;
use tauri::State;

use crate::error::AppError;
use crate::services::skill_store::{SkillStoreItem, SkillStoreService};

/// 列出商店技能输入参数。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ListStoreInput {
    /// 工作区根目录路径。
    pub workspace_path: String,
}

/// 安装技能输入参数。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct InstallSkillInput {
    /// 要安装的技能名称。
    pub skill_name: String,
    /// 工作区根目录路径。
    pub workspace_path: String,
}

/// 卸载技能输入参数。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct UninstallSkillInput {
    /// 要卸载的技能名称。
    pub skill_name: String,
}

/// 卸载技能响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct UninstallSkillResponse {
    /// 操作是否成功。
    pub success: bool,
}

/// 列出所有可用技能（已安装 + 已发现）。
#[tauri::command]
#[specta::specta]
pub async fn list_store_skills(
    pool: State<'_, SqlitePool>,
    input: ListStoreInput,
) -> Result<Vec<SkillStoreItem>, AppError> {
    let workspace_path = Path::new(&input.workspace_path);
    SkillStoreService::list_available_skills(&pool, workspace_path).await
}

/// 安装指定技能。
#[tauri::command]
#[specta::specta]
pub async fn install_store_skill(
    pool: State<'_, SqlitePool>,
    input: InstallSkillInput,
) -> Result<SkillStoreItem, AppError> {
    let workspace_path = Path::new(&input.workspace_path);
    SkillStoreService::install_skill(&pool, &input.skill_name, workspace_path).await
}

/// 卸载指定技能。
#[tauri::command]
#[specta::specta]
pub async fn uninstall_store_skill(
    pool: State<'_, SqlitePool>,
    input: UninstallSkillInput,
) -> Result<UninstallSkillResponse, AppError> {
    SkillStoreService::uninstall_skill(&pool, &input.skill_name).await?;
    Ok(UninstallSkillResponse { success: true })
}

//! 技能执行 IPC 命令模块
//!
//! 暴露技能执行与技能发现能力给前端调用。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use std::path::Path;
use tauri::State;

use crate::error::AppError;
use crate::services::path_whitelist::PathWhitelistService;
use crate::services::skill_discovery::{DiscoveredSkill, SkillDiscoveryService};
use crate::services::skill_executor::SkillExecutorService;
use crate::services::unified_tool::ToolResult;

/// 执行技能输入参数。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ExecuteSkillInput {
    /// 技能名称。
    pub skill_name: String,
    /// JSON 格式的技能输入参数。
    pub input: serde_json::Value,
}

/// 发现技能输入参数。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DiscoverSkillsInput {
    /// 工作区根目录路径。
    pub workspace_path: String,
}

/// 执行指定技能。
#[tauri::command]
#[specta::specta]
pub async fn execute_skill(
    pool: State<'_, SqlitePool>,
    input: ExecuteSkillInput,
) -> Result<ToolResult, AppError> {
    SkillExecutorService::execute_skill(&pool, &input.skill_name, input.input).await
}

/// 扫描并发现可用技能。
#[tauri::command]
#[specta::specta]
pub async fn discover_skills(
    pool: State<'_, SqlitePool>,
    input: DiscoverSkillsInput,
) -> Result<Vec<DiscoveredSkill>, AppError> {
    // 校验工作区路径合法性，防止前端传入任意路径扫描文件系统
    PathWhitelistService::validate_workspace_path(&input.workspace_path)?;
    let workspace_path = Path::new(&input.workspace_path);
    SkillDiscoveryService::discover_skills(&pool, workspace_path).await
}

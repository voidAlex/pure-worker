//! 技能注册 IPC 命令模块
//!
//! 暴露技能注册增删改查与健康检查能力给前端调用。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::skill::{CreateSkillInput, SkillHealthResult, SkillRecord, UpdateSkillInput};
use crate::services::skill::SkillService;

/// 删除技能输入。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteSkillInput {
    pub id: String,
}

/// 删除技能响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteSkillResponse {
    pub success: bool,
}

/// 列出技能。
#[tauri::command]
#[specta::specta]
pub async fn list_skills(pool: State<'_, SqlitePool>) -> Result<Vec<SkillRecord>, AppError> {
    SkillService::list_skills(&pool).await
}

/// 获取单个技能。
#[tauri::command]
#[specta::specta]
pub async fn get_skill(pool: State<'_, SqlitePool>, id: String) -> Result<SkillRecord, AppError> {
    SkillService::get_skill(&pool, &id).await
}

/// 创建技能。
#[tauri::command]
#[specta::specta]
pub async fn create_skill(
    pool: State<'_, SqlitePool>,
    input: CreateSkillInput,
) -> Result<SkillRecord, AppError> {
    SkillService::create_skill(&pool, input).await
}

/// 更新技能。
#[tauri::command]
#[specta::specta]
pub async fn update_skill(
    pool: State<'_, SqlitePool>,
    id: String,
    input: UpdateSkillInput,
) -> Result<SkillRecord, AppError> {
    SkillService::update_skill(&pool, &id, input).await
}

/// 删除技能。
#[tauri::command]
#[specta::specta]
pub async fn delete_skill(
    pool: State<'_, SqlitePool>,
    input: DeleteSkillInput,
) -> Result<DeleteSkillResponse, AppError> {
    SkillService::delete_skill(&pool, &input.id).await?;
    Ok(DeleteSkillResponse { success: true })
}

/// 检查技能健康状态。
#[tauri::command]
#[specta::specta]
pub async fn check_skill_health(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<SkillHealthResult, AppError> {
    SkillService::check_skill_health(&pool, &id).await
}

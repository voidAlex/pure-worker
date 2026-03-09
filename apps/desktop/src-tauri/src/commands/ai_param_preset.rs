//! AI 参数预设 IPC 命令模块
//!
//! 提供 AI 参数预设的增删改查与激活命令。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::ai_param_preset::{AiParamPreset, CreatePresetInput, UpdatePresetInput};
use crate::services;

/// 删除预设输入。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteAiParamPresetInput {
    pub id: String,
}

/// 删除预设响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteAiParamPresetResponse {
    pub success: bool,
}

/// 激活预设输入。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ActivateAiParamPresetInput {
    pub id: String,
}

/// 列出全部预设。
#[tauri::command]
#[specta::specta]
pub async fn list_ai_param_presets(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AiParamPreset>, AppError> {
    services::ai_param_preset::AiParamPresetService::list_presets(&pool).await
}

/// 获取当前激活预设。
#[tauri::command]
#[specta::specta]
pub async fn get_active_ai_param_preset(
    pool: State<'_, SqlitePool>,
) -> Result<AiParamPreset, AppError> {
    services::ai_param_preset::AiParamPresetService::get_active_preset(&pool).await
}

/// 创建预设。
#[tauri::command]
#[specta::specta]
pub async fn create_ai_param_preset(
    pool: State<'_, SqlitePool>,
    input: CreatePresetInput,
) -> Result<AiParamPreset, AppError> {
    services::ai_param_preset::AiParamPresetService::create_preset(&pool, input).await
}

/// 更新预设。
#[tauri::command]
#[specta::specta]
pub async fn update_ai_param_preset(
    pool: State<'_, SqlitePool>,
    input: UpdatePresetInput,
) -> Result<AiParamPreset, AppError> {
    services::ai_param_preset::AiParamPresetService::update_preset(&pool, input).await
}

/// 删除预设。
#[tauri::command]
#[specta::specta]
pub async fn delete_ai_param_preset(
    pool: State<'_, SqlitePool>,
    input: DeleteAiParamPresetInput,
) -> Result<DeleteAiParamPresetResponse, AppError> {
    services::ai_param_preset::AiParamPresetService::delete_preset(&pool, &input.id).await?;
    Ok(DeleteAiParamPresetResponse { success: true })
}

/// 激活预设。
#[tauri::command]
#[specta::specta]
pub async fn activate_ai_param_preset(
    pool: State<'_, SqlitePool>,
    input: ActivateAiParamPresetInput,
) -> Result<AiParamPreset, AppError> {
    services::ai_param_preset::AiParamPresetService::activate_preset(&pool, &input.id).await
}

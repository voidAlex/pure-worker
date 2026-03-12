//! AI 配置 IPC 命令
//!
//! 提供 AI Provider 配置的增删改查命令。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::ai_config::{
    AiConfigSafe, CreateAiConfigInput, ModelInfo, ProviderPreset, UpdateAiConfigInput,
};
use crate::services;

/// 删除 AI 配置输入。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteAiConfigInput {
    pub id: String,
}

/// 删除 AI 配置响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteAiConfigResponse {
    pub success: bool,
}

/// 列出全部 AI 配置。
#[tauri::command]
#[specta::specta]
pub async fn list_ai_configs(pool: State<'_, SqlitePool>) -> Result<Vec<AiConfigSafe>, AppError> {
    services::llm_provider::LlmProviderService::list_configs(&pool).await
}

/// 创建 AI 配置。
#[tauri::command]
#[specta::specta]
pub async fn create_ai_config(
    pool: State<'_, SqlitePool>,
    input: CreateAiConfigInput,
) -> Result<AiConfigSafe, AppError> {
    services::llm_provider::LlmProviderService::create_config(&pool, input).await
}

/// 更新 AI 配置。
#[tauri::command]
#[specta::specta]
pub async fn update_ai_config(
    pool: State<'_, SqlitePool>,
    input: UpdateAiConfigInput,
) -> Result<AiConfigSafe, AppError> {
    services::llm_provider::LlmProviderService::update_config(&pool, input).await
}

/// 删除 AI 配置（软删除）。
#[tauri::command]
#[specta::specta]
pub async fn delete_ai_config(
    pool: State<'_, SqlitePool>,
    input: DeleteAiConfigInput,
) -> Result<DeleteAiConfigResponse, AppError> {
    services::llm_provider::LlmProviderService::delete_config(&pool, &input.id).await?;

    Ok(DeleteAiConfigResponse { success: true })
}

/// 获取供应商可用模型列表。
#[tauri::command]
#[specta::specta]
pub async fn fetch_provider_models(
    provider_name: String,
    base_url: String,
    api_key: String,
) -> Result<Vec<ModelInfo>, AppError> {
    services::llm_provider::fetch_provider_models(&provider_name, &base_url, &api_key).await
}

/// 获取供应商预设配置列表。
#[tauri::command]
#[specta::specta]
pub async fn get_provider_presets() -> Result<Vec<ProviderPreset>, AppError> {
    Ok(services::llm_provider::get_provider_presets())
}

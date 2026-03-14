//! 教师偏好记忆 IPC 命令模块
//!
//! 暴露教师偏好管理、soul.md/user.md 文件管理、候选记忆处理等 IPC 命令。

use sqlx::SqlitePool;
use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::models::teacher_memory::{
    ConfirmCandidateInput, ListCandidatesInput, MemoryCandidate, RejectCandidateInput,
    ReloadSoulMdInput, SetPreferenceInput, SoulMdContent, SystemPromptContext, TeacherPreference,
};
use crate::services::soul_md_manager::SoulMdManager;
use crate::services::teacher_memory::TeacherMemoryService;

/// 获取 workspace 路径
fn get_workspace_path(app_handle: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::FileOperation(format!("获取数据目录失败: {}", e)))?;
    Ok(data_dir.join("workspace"))
}

/// 获取所有教师偏好
#[tauri::command]
#[specta::specta]
pub async fn get_teacher_preferences(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<TeacherPreference>, AppError> {
    TeacherMemoryService::list_preferences(&pool).await
}

/// 设置教师偏好
#[tauri::command]
#[specta::specta]
pub async fn set_teacher_preference(
    pool: State<'_, SqlitePool>,
    input: SetPreferenceInput,
) -> Result<TeacherPreference, AppError> {
    TeacherMemoryService::set_preference(&pool, &input).await
}

/// 删除教师偏好
#[tauri::command]
#[specta::specta]
pub async fn delete_teacher_preference(
    pool: State<'_, SqlitePool>,
    key: String,
) -> Result<(), AppError> {
    TeacherMemoryService::delete_preference(&pool, &key).await
}

/// 列出候选记忆
#[tauri::command]
#[specta::specta]
pub async fn list_memory_candidates(
    pool: State<'_, SqlitePool>,
    input: ListCandidatesInput,
) -> Result<Vec<MemoryCandidate>, AppError> {
    TeacherMemoryService::list_candidates(&pool, &input).await
}

/// 确认候选记忆
#[tauri::command]
#[specta::specta]
pub async fn confirm_memory_candidate(
    pool: State<'_, SqlitePool>,
    input: ConfirmCandidateInput,
) -> Result<TeacherPreference, AppError> {
    TeacherMemoryService::confirm_candidate(&pool, &input).await
}

/// 拒绝候选记忆
#[tauri::command]
#[specta::specta]
pub async fn reject_memory_candidate(
    pool: State<'_, SqlitePool>,
    input: RejectCandidateInput,
) -> Result<(), AppError> {
    TeacherMemoryService::reject_candidate(&pool, &input).await
}

/// 加载 soul.md 文件
#[tauri::command]
#[specta::specta]
pub fn load_soul_md(app_handle: AppHandle) -> Result<SoulMdContent, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    SoulMdManager::load_soul_md(&workspace_path)
}

/// 加载 user.md 文件
#[tauri::command]
#[specta::specta]
pub fn load_user_md(app_handle: AppHandle) -> Result<SoulMdContent, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    SoulMdManager::load_user_md(&workspace_path)
}

/// 重新加载 soul.md（可选强制重新创建）
#[tauri::command]
#[specta::specta]
pub fn reload_soul_md(
    app_handle: AppHandle,
    input: ReloadSoulMdInput,
) -> Result<SoulMdContent, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    let force_create = input.force_create.unwrap_or(false);
    SoulMdManager::reload_soul_md(&workspace_path, force_create)
}

/// 获取系统提示词上下文（包含偏好注入）
#[tauri::command]
#[specta::specta]
pub async fn build_system_prompt_context(
    pool: State<'_, SqlitePool>,
) -> Result<SystemPromptContext, AppError> {
    TeacherMemoryService::build_system_prompt_context(&pool).await
}

/// 记录模式检测
#[tauri::command]
#[specta::specta]
pub async fn record_preference_pattern(
    pool: State<'_, SqlitePool>,
    pattern_type: String,
    pattern_key: String,
    pattern_value: Option<String>,
    context: Option<String>,
) -> Result<bool, AppError> {
    let context_hash = context.as_ref().map(|c| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        c.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    });

    TeacherMemoryService::record_pattern_detection(
        &pool,
        &pattern_type,
        &pattern_key,
        pattern_value.as_deref(),
        context_hash.as_deref(),
    )
    .await
}

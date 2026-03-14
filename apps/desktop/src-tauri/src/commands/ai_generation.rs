//! AI 生成 IPC 命令模块
//!
//! 暴露家长沟通文案、学期评语、活动公告的 AI 生成命令。

use sqlx::SqlitePool;
use tauri::{Manager, State};

use crate::error::AppError;
use crate::models::activity_announcement::ActivityAnnouncement;
use crate::models::async_task::AsyncTask;
use crate::models::parent_communication::ParentCommunication;
use crate::models::semester_comment::SemesterComment;
use crate::services::ai_generation::{
    AiGenerationService, GenerateActivityAnnouncementInput, GenerateBatchCommentsInput,
    GenerateParentCommInput, GenerateSemesterCommentInput, RegenerateParentCommInput,
};
use crate::services::async_task::AsyncTaskService;

/// 获取工作区路径。
fn get_workspace_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| AppError::FileOperation(format!("获取数据目录失败：{error}")))?;
    Ok(data_dir.join("workspace"))
}

/// 获取提示词模板目录。
fn get_templates_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("..")
        .join("..")
        .join("..")
        .join("packages")
        .join("prompt-templates")
}

/// 生成家长沟通文案。
#[tauri::command]
#[specta::specta]
pub async fn generate_parent_communication(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: GenerateParentCommInput,
) -> Result<ParentCommunication, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    let templates_dir = get_templates_dir();
    AiGenerationService::generate_parent_communication(
        &pool,
        &workspace_path,
        &templates_dir,
        input,
    )
    .await
}

/// 重新生成家长沟通文案。
#[tauri::command]
#[specta::specta]
pub async fn regenerate_parent_communication(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: RegenerateParentCommInput,
) -> Result<ParentCommunication, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    let templates_dir = get_templates_dir();
    AiGenerationService::regenerate_parent_communication(
        &pool,
        &workspace_path,
        &templates_dir,
        input,
    )
    .await
}

/// 生成单个学期评语。
#[tauri::command]
#[specta::specta]
pub async fn generate_semester_comment(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: GenerateSemesterCommentInput,
) -> Result<SemesterComment, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    let templates_dir = get_templates_dir();
    AiGenerationService::generate_semester_comment(&pool, &workspace_path, &templates_dir, input)
        .await
}

/// 启动批量学期评语生成并异步执行。
#[tauri::command]
#[specta::specta]
pub async fn generate_semester_comments_batch(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: GenerateBatchCommentsInput,
) -> Result<AsyncTask, AppError> {
    let task = AiGenerationService::start_batch_semester_comments(&pool, input.clone()).await?;

    let pool_clone = pool.inner().clone();
    let workspace_path = get_workspace_path(&app_handle)?;
    let templates_dir = get_templates_dir();
    let task_id = task.id.clone();

    tokio::spawn(async move {
        let run_result = AiGenerationService::run_batch_semester_comments(
            &pool_clone,
            &workspace_path,
            &templates_dir,
            &task_id,
            input,
        )
        .await;

        if let Err(error) = run_result {
            let _ = AsyncTaskService::fail(
                &pool_clone,
                &task_id,
                "batch_semester_comments_failed",
                &error.to_string(),
            )
            .await;
        }
    });

    Ok(task)
}

/// 获取批量任务进度。
#[tauri::command]
#[specta::specta]
pub async fn get_batch_task_progress(
    pool: State<'_, SqlitePool>,
    task_id: String,
) -> Result<AsyncTask, AppError> {
    AsyncTaskService::get_by_id(&pool, &task_id).await
}

/// 生成活动公告。
#[tauri::command]
#[specta::specta]
pub async fn generate_activity_announcement(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: GenerateActivityAnnouncementInput,
) -> Result<ActivityAnnouncement, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    let templates_dir = get_templates_dir();
    let template_file_dir = workspace_path.join("templates");
    AiGenerationService::generate_activity_announcement(
        &pool,
        &workspace_path,
        &templates_dir,
        &template_file_dir,
        input,
    )
    .await
}

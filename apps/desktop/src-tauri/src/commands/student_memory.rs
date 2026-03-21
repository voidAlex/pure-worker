//! 学生长期记忆 IPC 命令模块。
//!
//! 暴露学生长期记忆初始化、读取、追加及敏感信息检测命令。

use serde::Deserialize;
use specta::Type;

use crate::error::AppError;
use crate::models::student_memory::{
    AppendMemoryNoteInput, InitStudentMemoryInput, MemoryEntry, ReadCommentMaterialsInput,
    ReadMemoryByTopicInput, ReadMemoryTimelineInput, SensitiveInfoResult,
};
use crate::services::runtime_paths;
use crate::services::student_memory;

/// 文本敏感信息检测输入。
#[derive(Debug, Deserialize, Type)]
pub struct CheckSensitiveInput {
    /// 待检测文本内容。
    pub content: String,
}

fn get_workspace_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    runtime_paths::resolve_workspace_path(app_handle)
}

/// 初始化学生长期记忆目录与当月模板。
#[tauri::command]
#[specta::specta]
pub fn init_student_memory(
    app_handle: tauri::AppHandle,
    input: InitStudentMemoryInput,
) -> Result<String, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    let file_path = student_memory::init_student_memory(&workspace_path, &input)?;
    Ok(file_path.to_string_lossy().to_string())
}

/// 读取学生记忆时间线。
#[tauri::command]
#[specta::specta]
pub fn read_student_memory_timeline(
    app_handle: tauri::AppHandle,
    input: ReadMemoryTimelineInput,
) -> Result<Vec<MemoryEntry>, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    student_memory::read_memory_timeline(&workspace_path, &input)
}

/// 按主题读取学生记忆。
#[tauri::command]
#[specta::specta]
pub fn read_student_memory_by_topic(
    app_handle: tauri::AppHandle,
    input: ReadMemoryByTopicInput,
) -> Result<Vec<MemoryEntry>, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    student_memory::read_memory_by_topic(&workspace_path, &input)
}

/// 读取学生评语素材池条目。
#[tauri::command]
#[specta::specta]
pub fn read_student_comment_materials(
    app_handle: tauri::AppHandle,
    input: ReadCommentMaterialsInput,
) -> Result<Vec<MemoryEntry>, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    student_memory::read_comment_materials(&workspace_path, &input)
}

/// 追加学生记忆笔记到目标章节。
#[tauri::command]
#[specta::specta]
pub fn append_student_memory_note(
    app_handle: tauri::AppHandle,
    input: AppendMemoryNoteInput,
) -> Result<(), AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    student_memory::append_memory_note(&workspace_path, &input)
}

/// 检测文本是否包含敏感信息。
#[tauri::command]
#[specta::specta]
pub fn check_sensitive_content(
    input: CheckSensitiveInput,
) -> Result<SensitiveInfoResult, AppError> {
    Ok(student_memory::check_sensitive_info(&input.content))
}

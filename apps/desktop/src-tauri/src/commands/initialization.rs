//! 初始化状态检查 IPC 命令模块
//!
//! 提供首次启动向导所需的初始化状态检查与目录选择命令。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::error::AppError;

/// 初始化状态响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct InitializationStatus {
    /// 是否已完成初始化
    pub initialized: bool,
    /// 是否已设置工作目录（非默认值）
    pub has_workspace: bool,
    /// 是否已配置至少一个 AI 供应商
    pub has_ai_config: bool,
}

/// 检查首次启动初始化状态。
#[tauri::command]
#[specta::specta]
pub async fn check_initialization_status(
    pool: State<'_, SqlitePool>,
) -> Result<InitializationStatus, AppError> {
    let initialization_value = sqlx::query_scalar::<_, String>(
        "SELECT value FROM app_settings WHERE key = ? AND is_deleted = 0",
    )
    .bind("initialization_completed")
    .fetch_optional(&*pool)
    .await?;

    let workspace_value = sqlx::query_scalar::<_, String>(
        "SELECT value FROM app_settings WHERE key = ? AND is_deleted = 0",
    )
    .bind("workspace_path")
    .fetch_optional(&*pool)
    .await?;

    let ai_config_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM ai_config WHERE is_deleted = 0")
            .fetch_one(&*pool)
            .await?;

    let initialized = initialization_value.as_deref() == Some("true");
    let has_workspace = workspace_value
        .as_deref()
        .is_some_and(|value| value != "./workspace");
    let has_ai_config = ai_config_count > 0;

    Ok(InitializationStatus {
        initialized,
        has_workspace,
        has_ai_config,
    })
}

/// 打开系统原生目录选择对话框。
///
/// 返回用户选择的目录路径字符串，若用户取消则返回 None。
#[tauri::command]
#[specta::specta]
pub async fn select_directory(app: tauri::AppHandle) -> Result<Option<String>, AppError> {
    let selected = app
        .dialog()
        .file()
        .blocking_pick_folder()
        .map(|path| path.to_string());

    Ok(selected)
}

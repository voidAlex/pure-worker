use crate::error::AppError;
use crate::services::runtime_paths;

/// 获取应用程序日志目录路径
///
/// 返回日志文件夹的完整路径，用于前端显示和排查问题
#[tauri::command]
#[specta::specta]
pub fn get_log_path(app_handle: tauri::AppHandle) -> Result<String, AppError> {
    let workspace_path = runtime_paths::resolve_workspace_path(&app_handle)?;
    runtime_paths::ensure_workspace_layout(&workspace_path)?;
    let log_dir = runtime_paths::log_dir_path(&workspace_path);

    // 确保日志目录存在
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        return Err(AppError::Internal(format!("无法创建日志目录：{}", e)));
    }

    Ok(log_dir.to_string_lossy().to_string())
}

use tauri::Manager;

use crate::error::AppError;

/// 获取应用程序日志目录路径
///
/// 返回日志文件夹的完整路径，用于前端显示和排查问题
#[tauri::command]
#[specta::specta]
pub fn get_log_path(app_handle: tauri::AppHandle) -> Result<String, AppError> {
    let log_dir = app_handle
        .path()
        .app_log_dir()
        .map_err(|e| AppError::Internal(format!("无法获取日志目录：{}", e)))?;

    // 确保日志目录存在
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        return Err(AppError::Internal(format!("无法创建日志目录：{}", e)));
    }

    Ok(log_dir.to_string_lossy().to_string())
}

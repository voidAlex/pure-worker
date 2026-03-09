//! 监控文件夹数据模型
//!
//! 定义监控文件夹记录及创建/更新输入结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 监控文件夹记录。
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct WatchFolder {
    pub id: String,
    pub folder_path: String,
    pub pattern: Option<String>,
    pub action: String,
    pub enabled: i32,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建监控文件夹输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateWatchFolderInput {
    pub folder_path: String,
    pub pattern: Option<String>,
    pub action: String,
    pub enabled: Option<i32>,
}

/// 更新监控文件夹输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdateWatchFolderInput {
    pub id: String,
    pub folder_path: Option<String>,
    pub pattern: Option<String>,
    pub action: Option<String>,
    pub enabled: Option<i32>,
}

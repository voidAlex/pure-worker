//! 存储生命周期数据模型。
//!
//! 定义工作区存储统计返回结构。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 工作区存储统计信息。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct StorageStats {
    /// 工作区路径。
    pub workspace_path: String,
    /// 工作区文件总数。
    pub total_files: u64,
    /// 工作区总字节数。
    pub total_size_bytes: u64,
    /// 人类可读的存储大小展示。
    pub total_size_display: String,
    /// 归档目录总数。
    pub archive_count: u64,
}

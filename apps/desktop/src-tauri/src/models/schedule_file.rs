//! 课表文件数据模型
//!
//! 定义课表事件关联文件的数据结构

use serde::{Deserialize, Serialize};
use specta::Type;

/// 课表事件关联的文件（教案/课件等）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct ScheduleFile {
    pub id: String,
    pub class_id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_type: Option<String>,
    pub file_size: Option<i64>,
    pub is_deleted: i32,
    pub created_at: String,
}

/// 创建课表文件输入参数
#[derive(Debug, Deserialize, Type)]
pub struct CreateScheduleFileInput {
    pub class_id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_type: Option<String>,
    pub file_size: Option<i64>,
}

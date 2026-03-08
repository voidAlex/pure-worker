//! 异步任务数据模型
//!
//! 定义异步任务的结构体及状态管理输入类型。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 异步任务记录。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct AsyncTask {
    pub id: String,
    pub task_type: String,
    pub target_id: Option<String>,
    pub status: String,
    pub progress_json: Option<String>,
    pub context_data: Option<String>,
    pub checkpoint_cursor: Option<String>,
    pub completed_items_json: Option<String>,
    pub partial_output_path: Option<String>,
    pub lease_until: Option<String>,
    pub attempt_count: i32,
    pub last_heartbeat_at: Option<String>,
    pub worker_id: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建异步任务输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateAsyncTaskInput {
    pub task_type: String,
    pub target_id: Option<String>,
    pub context_data: Option<String>,
}

/// 批量评语生成进度。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BatchProgress {
    pub total: i32,
    pub completed: i32,
    pub failed: i32,
    pub current_student_name: Option<String>,
}

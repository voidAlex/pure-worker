//! 审批请求数据模型
//!
//! 定义人工确认审批请求的结构体及输入类型。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 审批请求记录（对应 approval_request 表）。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct ApprovalRequest {
    /// 审批请求主键 ID。
    pub id: String,
    /// 关联异步任务 ID。
    pub task_id: Option<String>,
    /// 审批请求类型。
    pub request_type: String,
    /// 操作摘要。
    pub action_summary: String,
    /// 参数预览（JSON 字符串）。
    pub params_preview: Option<String>,
    /// 风险等级（low/medium/high）。
    pub risk_level: String,
    /// 当前状态（pending/approved/rejected/expired）。
    pub status: String,
    /// 审批处理人。
    pub resolved_by: Option<String>,
    /// 审批处理时间。
    pub resolved_at: Option<String>,
    /// 超时时间。
    pub timeout_at: String,
    /// 创建时间。
    pub created_at: String,
    /// 更新时间。
    pub updated_at: Option<String>,
}

/// 创建审批请求输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateApprovalInput {
    /// 关联异步任务 ID。
    pub task_id: Option<String>,
    /// 审批请求类型。
    pub request_type: String,
    /// 操作摘要。
    pub action_summary: String,
    /// 参数预览（JSON 字符串）。
    pub params_preview: Option<String>,
    /// 风险等级（low/medium/high）。
    pub risk_level: String,
    /// 超时时间（分钟），超时后自动拒绝。
    pub timeout_minutes: i64,
}

/// 解决审批请求输入。
#[derive(Debug, Deserialize, Type)]
pub struct ResolveApprovalInput {
    /// 审批请求 ID。
    pub request_id: String,
    /// 处理结果："approved" 或 "rejected"。
    pub decision: String,
    /// 审批处理人。
    pub resolved_by: Option<String>,
}

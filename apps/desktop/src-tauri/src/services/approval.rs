//! 审批请求服务模块
//!
//! 提供审批请求的创建、查询、解决与过期清理能力。

use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::approval_request::{ApprovalRequest, CreateApprovalInput, ResolveApprovalInput};
use crate::services::audit::AuditService;

/// 审批请求服务。
pub struct ApprovalService;

impl ApprovalService {
    /// 创建审批请求，状态默认为 pending。
    pub async fn create(
        pool: &SqlitePool,
        input: CreateApprovalInput,
    ) -> Result<ApprovalRequest, AppError> {
        if input.request_type.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from(
                "request_type 不能为空",
            )));
        }
        if input.action_summary.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from(
                "action_summary 不能为空",
            )));
        }
        if input.risk_level.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("risk_level 不能为空")));
        }
        if input.timeout_minutes <= 0 {
            return Err(AppError::InvalidInput(String::from(
                "timeout_minutes 必须大于 0",
            )));
        }

        let id = Uuid::new_v4().to_string();
        let now_dt = Utc::now();
        let now = now_dt.to_rfc3339();
        let timeout_at = (now_dt + Duration::minutes(input.timeout_minutes)).to_rfc3339();

        sqlx::query(
            "INSERT INTO approval_request (id, task_id, request_type, action_summary, params_preview, risk_level, status, resolved_by, resolved_at, timeout_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 'pending', NULL, NULL, ?, ?, NULL)",
        )
        .bind(&id)
        .bind(&input.task_id)
        .bind(&input.request_type)
        .bind(&input.action_summary)
        .bind(&input.params_preview)
        .bind(&input.risk_level)
        .bind(&timeout_at)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_approval_request",
            "approval_request",
            Some(&id),
            &input.risk_level,
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 根据审批请求 ID 查询详情。
    pub async fn get_by_id(
        pool: &SqlitePool,
        request_id: &str,
    ) -> Result<ApprovalRequest, AppError> {
        let request = sqlx::query_as::<_, ApprovalRequest>(
            "SELECT id, task_id, request_type, action_summary, params_preview, risk_level, status, resolved_by, resolved_at, timeout_at, created_at, updated_at FROM approval_request WHERE id = ?",
        )
        .bind(request_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("审批请求不存在：{request_id}")))?;

        Ok(request)
    }

    /// 查询全部待处理审批请求。
    pub async fn list_pending(pool: &SqlitePool) -> Result<Vec<ApprovalRequest>, AppError> {
        let requests = sqlx::query_as::<_, ApprovalRequest>(
            "SELECT id, task_id, request_type, action_summary, params_preview, risk_level, status, resolved_by, resolved_at, timeout_at, created_at, updated_at FROM approval_request WHERE status = 'pending' ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(requests)
    }

    /// 解决待处理审批请求（批准或拒绝）。
    pub async fn resolve(
        pool: &SqlitePool,
        input: ResolveApprovalInput,
    ) -> Result<ApprovalRequest, AppError> {
        if input.decision != "approved" && input.decision != "rejected" {
            return Err(AppError::InvalidInput(String::from(
                "decision 必须为 approved 或 rejected",
            )));
        }

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE approval_request SET status = ?, resolved_by = ?, resolved_at = ?, updated_at = ? WHERE id = ? AND status = 'pending'",
        )
        .bind(&input.decision)
        .bind(&input.resolved_by)
        .bind(&now)
        .bind(&now)
        .bind(&input.request_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            let exists =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM approval_request WHERE id = ?")
                    .bind(&input.request_id)
                    .fetch_one(pool)
                    .await?;

            if exists == 0 {
                return Err(AppError::NotFound(format!(
                    "审批请求不存在：{}",
                    input.request_id
                )));
            }

            return Err(AppError::InvalidInput(String::from("审批请求已处理")));
        }

        AuditService::log(
            pool,
            "system",
            "resolve_approval_request",
            "approval_request",
            Some(&input.request_id),
            "high",
            true,
        )
        .await?;

        Self::get_by_id(pool, &input.request_id).await
    }

    /// 清理已过期且仍处于待处理状态的审批请求。
    pub async fn cleanup_expired(pool: &SqlitePool) -> Result<i32, AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE approval_request SET status = 'expired', updated_at = ? WHERE status = 'pending' AND timeout_at < ?",
        )
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let count = i32::try_from(result.rows_affected())
            .map_err(|_| AppError::Internal(String::from("过期审批清理数量超出 i32 范围")))?;

        AuditService::log(
            pool,
            "system",
            "cleanup_expired_approvals",
            "approval_request",
            None,
            "medium",
            false,
        )
        .await?;

        Ok(count)
    }

    /// 启动恢复时查询仍可恢复的待处理审批请求。
    pub async fn list_pending_for_restore(
        pool: &SqlitePool,
    ) -> Result<Vec<ApprovalRequest>, AppError> {
        let now = Utc::now().to_rfc3339();
        let requests = sqlx::query_as::<_, ApprovalRequest>(
            "SELECT id, task_id, request_type, action_summary, params_preview, risk_level, status, resolved_by, resolved_at, timeout_at, created_at, updated_at FROM approval_request WHERE status = 'pending' AND timeout_at > ? ORDER BY created_at ASC",
        )
        .bind(&now)
        .fetch_all(pool)
        .await?;

        Ok(requests)
    }

    /// 查询指定任务的审批记录。
    pub async fn list_by_task(
        pool: &SqlitePool,
        task_id: &str,
    ) -> Result<Vec<ApprovalRequest>, AppError> {
        let requests = sqlx::query_as::<_, ApprovalRequest>(
            "SELECT id, task_id, request_type, action_summary, params_preview, risk_level, status, resolved_by, resolved_at, timeout_at, created_at, updated_at FROM approval_request WHERE task_id = ? ORDER BY created_at DESC",
        )
        .bind(task_id)
        .fetch_all(pool)
        .await?;

        Ok(requests)
    }
}

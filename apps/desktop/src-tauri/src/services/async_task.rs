//! 异步任务服务模块
//!
//! 提供异步任务的创建、启动、进度更新、完成、失败和查询能力。

use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::async_task::{AsyncTask, CreateAsyncTaskInput};
use crate::services::audit::AuditService;

/// 异步任务服务。
pub struct AsyncTaskService;

impl AsyncTaskService {
    /// 创建异步任务，初始状态为 pending。
    pub async fn create(
        pool: &SqlitePool,
        input: CreateAsyncTaskInput,
    ) -> Result<AsyncTask, AppError> {
        if input.task_type.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("task_type 不能为空")));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO async_task (id, task_type, target_id, status, progress_json, context_data, checkpoint_cursor, completed_items_json, partial_output_path, lease_until, attempt_count, last_heartbeat_at, worker_id, error_code, error_message, created_at, updated_at) VALUES (?, ?, ?, 'pending', NULL, ?, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, NULL, ?, ?)",
        )
        .bind(&id)
        .bind(&input.task_type)
        .bind(&input.target_id)
        .bind(&input.context_data)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_async_task",
            "async_task",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 启动异步任务，状态切换为 running 并设置 10 分钟租约。
    pub async fn start(pool: &SqlitePool, task_id: &str) -> Result<AsyncTask, AppError> {
        let now_dt = Utc::now();
        let now = now_dt.to_rfc3339();
        let lease_until = (now_dt + Duration::minutes(10)).to_rfc3339();

        let result = sqlx::query(
            "UPDATE async_task SET status = 'running', lease_until = ?, last_heartbeat_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&lease_until)
        .bind(&now)
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("异步任务不存在：{task_id}")));
        }

        AuditService::log(
            pool,
            "system",
            "start_async_task",
            "async_task",
            Some(task_id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, task_id).await
    }

    /// 更新异步任务进度并刷新心跳时间。
    pub async fn update_progress(
        pool: &SqlitePool,
        task_id: &str,
        progress_json: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE async_task SET progress_json = ?, last_heartbeat_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(progress_json)
        .bind(&now)
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("异步任务不存在：{task_id}")));
        }

        Ok(())
    }

    /// 将异步任务标记为 completed，并写入完成项信息。
    pub async fn complete(
        pool: &SqlitePool,
        task_id: &str,
        completed_items_json: Option<&str>,
    ) -> Result<AsyncTask, AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE async_task SET status = 'completed', completed_items_json = ?, lease_until = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(completed_items_json)
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("异步任务不存在：{task_id}")));
        }

        AuditService::log(
            pool,
            "system",
            "complete_async_task",
            "async_task",
            Some(task_id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, task_id).await
    }

    /// 将异步任务标记为 failed，并记录错误信息。
    pub async fn fail(
        pool: &SqlitePool,
        task_id: &str,
        error_code: &str,
        error_message: &str,
    ) -> Result<AsyncTask, AppError> {
        if error_code.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("error_code 不能为空")));
        }
        if error_message.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from(
                "error_message 不能为空",
            )));
        }

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE async_task SET status = 'failed', error_code = ?, error_message = ?, lease_until = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(error_code)
        .bind(error_message)
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("异步任务不存在：{task_id}")));
        }

        AuditService::log(
            pool,
            "system",
            "fail_async_task",
            "async_task",
            Some(task_id),
            "high",
            false,
        )
        .await?;

        Self::get_by_id(pool, task_id).await
    }

    /// 根据任务 ID 查询异步任务详情。
    pub async fn get_by_id(pool: &SqlitePool, task_id: &str) -> Result<AsyncTask, AppError> {
        let task = sqlx::query_as::<_, AsyncTask>(
            "SELECT id, task_type, target_id, status, progress_json, context_data, checkpoint_cursor, completed_items_json, partial_output_path, lease_until, attempt_count, last_heartbeat_at, worker_id, error_code, error_message, created_at, updated_at FROM async_task WHERE id = ?",
        )
        .bind(task_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("异步任务不存在：{task_id}")))?;

        Ok(task)
    }
}

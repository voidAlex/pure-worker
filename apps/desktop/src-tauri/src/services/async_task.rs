//! 异步任务服务模块
//!
//! 提供异步任务的创建、启动、进度更新、完成、失败和查询能力。

use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::async_task::{AsyncTask, CreateAsyncTaskInput, RecoverableTask};
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

    /// 创建异步任务并持久化完整输入快照。
    pub async fn create_with_snapshot(
        pool: &SqlitePool,
        input: CreateAsyncTaskInput,
        input_snapshot: &str,
    ) -> Result<AsyncTask, AppError> {
        if input.task_type.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("task_type 不能为空")));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO async_task (id, task_type, target_id, status, progress_json, context_data, checkpoint_cursor, completed_items_json, partial_output_path, lease_until, attempt_count, last_heartbeat_at, worker_id, error_code, error_message, input_snapshot, created_at, updated_at) VALUES (?, ?, ?, 'pending', NULL, ?, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, NULL, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&input.task_type)
        .bind(&input.target_id)
        .bind(&input.context_data)
        .bind(input_snapshot)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_async_task_with_snapshot",
            "async_task",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 保存任务检查点（幂等）。
    pub async fn save_checkpoint(
        pool: &SqlitePool,
        task_id: &str,
        item_id: &str,
        result_json: Option<&str>,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let checkpoint_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT OR IGNORE INTO task_checkpoint_item (id, task_id, item_id, status, result_json, created_at) VALUES (?, ?, ?, 'completed', ?, ?)",
        )
        .bind(&checkpoint_id)
        .bind(task_id)
        .bind(item_id)
        .bind(result_json)
        .bind(&now)
        .execute(pool)
        .await?;

        let result = sqlx::query(
            "UPDATE async_task SET checkpoint_cursor = ?, last_heartbeat_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(item_id)
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

    /// 查询可恢复任务列表。
    pub async fn find_recoverable_tasks(
        pool: &SqlitePool,
    ) -> Result<Vec<RecoverableTask>, AppError> {
        let tasks = sqlx::query_as::<_, RecoverableTask>(
            "SELECT t.id, t.task_type, t.status, t.checkpoint_cursor, COALESCE(c.completed_items_count, 0) AS completed_items_count, t.attempt_count, t.created_at FROM async_task t LEFT JOIN (SELECT task_id, COUNT(*) AS completed_items_count FROM task_checkpoint_item WHERE status = 'completed' GROUP BY task_id) c ON t.id = c.task_id WHERE t.status IN ('running', 'recovering') ORDER BY t.created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(tasks)
    }

    /// 将任务状态标记为 recovering。
    pub async fn mark_recovering(pool: &SqlitePool, task_id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE async_task SET status = 'recovering', recovering_since = ?, attempt_count = attempt_count + 1, updated_at = ? WHERE id = ?",
        )
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

    /// 查询已完成检查点 item_id 列表。
    pub async fn get_completed_item_ids(
        pool: &SqlitePool,
        task_id: &str,
    ) -> Result<Vec<String>, AppError> {
        let item_ids = sqlx::query_scalar::<_, String>(
            "SELECT item_id FROM task_checkpoint_item WHERE task_id = ? AND status = 'completed'",
        )
        .bind(task_id)
        .fetch_all(pool)
        .await?;

        Ok(item_ids)
    }

    /// 恢复任务为 running 并续租。
    pub async fn resume_task(pool: &SqlitePool, task_id: &str) -> Result<AsyncTask, AppError> {
        let now_dt = Utc::now();
        let now = now_dt.to_rfc3339();
        let lease_until = (now_dt + Duration::minutes(10)).to_rfc3339();
        let worker_id = Uuid::new_v4().to_string();

        let result = sqlx::query(
            "UPDATE async_task SET status = 'running', lease_until = ?, worker_id = ?, recovering_since = NULL, last_heartbeat_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&lease_until)
        .bind(&worker_id)
        .bind(&now)
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("异步任务不存在：{task_id}")));
        }

        Self::get_by_id(pool, task_id).await
    }

    /// 取消恢复中的任务。
    pub async fn cancel_recovering_task(
        pool: &SqlitePool,
        task_id: &str,
    ) -> Result<AsyncTask, AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE async_task SET status = 'cancelled', lease_until = NULL, worker_id = NULL, recovering_since = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("异步任务不存在：{task_id}")));
        }

        Self::get_by_id(pool, task_id).await
    }

    /// 续租任务租约并刷新心跳。
    pub async fn renew_lease(
        pool: &SqlitePool,
        task_id: &str,
        minutes: i64,
    ) -> Result<(), AppError> {
        if minutes <= 0 {
            return Err(AppError::InvalidInput(String::from("minutes 必须大于 0")));
        }

        let now_dt = Utc::now();
        let now = now_dt.to_rfc3339();
        let lease_until = (now_dt + Duration::minutes(minutes)).to_rfc3339();

        let result = sqlx::query(
            "UPDATE async_task SET lease_until = ?, last_heartbeat_at = ?, updated_at = ? WHERE id = ? AND status = 'running'",
        )
        .bind(&lease_until)
        .bind(&now)
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "异步任务不存在或不是运行中状态：{task_id}",
            )));
        }

        Ok(())
    }

    /// 查询任务列表，可选按状态过滤。
    pub async fn list_tasks(
        pool: &SqlitePool,
        status_filter: Option<&str>,
    ) -> Result<Vec<AsyncTask>, AppError> {
        let tasks = if let Some(status) = status_filter {
            sqlx::query_as::<_, AsyncTask>(
                "SELECT id, task_type, target_id, status, progress_json, context_data, checkpoint_cursor, completed_items_json, partial_output_path, lease_until, attempt_count, last_heartbeat_at, worker_id, error_code, error_message, created_at, updated_at FROM async_task WHERE status = ? ORDER BY created_at DESC",
            )
            .bind(status)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, AsyncTask>(
                "SELECT id, task_type, target_id, status, progress_json, context_data, checkpoint_cursor, completed_items_json, partial_output_path, lease_until, attempt_count, last_heartbeat_at, worker_id, error_code, error_message, created_at, updated_at FROM async_task ORDER BY created_at DESC",
            )
            .fetch_all(pool)
            .await?
        };

        Ok(tasks)
    }
}

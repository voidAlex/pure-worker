//! 批量子执行模型
//!
//! 【WP-AI-BIZ-005】学期评语批量任务运行时化
//! 定义批量任务中每个子项的执行记录，支持子执行聚合模型

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;

/// 批量子执行状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum BatchSubExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for BatchSubExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// 批量子执行记录
///
/// 表示批量任务中每个子项（如每个学生）的独立执行记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BatchSubExecution {
    pub id: String,
    pub parent_task_id: String, // 关联 AsyncTask
    pub student_id: String,
    pub student_name: String,
    pub status: String, // pending/running/completed/failed
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub result_comment_id: Option<String>,   // 生成的评语 ID
    pub execution_record_id: Option<String>, // 关联 execution_record
    pub created_at: String,
    pub updated_at: String,
}

/// 创建子执行输入
#[derive(Debug, Deserialize)]
pub struct CreateBatchSubExecutionInput {
    pub parent_task_id: String,
    pub student_id: String,
    pub student_name: String,
}

/// 更新子执行输入
#[derive(Debug, Deserialize)]
pub struct UpdateBatchSubExecutionInput {
    pub id: String,
    pub status: Option<BatchSubExecutionStatus>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub result_comment_id: Option<String>,
    pub execution_record_id: Option<String>,
}

/// 子执行查询过滤条件
#[derive(Debug, Deserialize)]
pub struct ListBatchSubExecutionsInput {
    pub parent_task_id: String,
    pub status: Option<BatchSubExecutionStatus>,
}

/// 批量子执行服务
pub struct BatchSubExecutionService;

impl BatchSubExecutionService {
    /// 创建子执行记录
    pub async fn create(
        pool: &SqlitePool,
        input: CreateBatchSubExecutionInput,
    ) -> Result<BatchSubExecution, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO batch_sub_execution (id, parent_task_id, student_id, student_name, status, error_code, error_message, result_comment_id, execution_record_id, created_at, updated_at) VALUES (?, ?, ?, ?, 'pending', NULL, NULL, NULL, NULL, ?, ?)",
        )
        .bind(&id)
        .bind(&input.parent_task_id)
        .bind(&input.student_id)
        .bind(&input.student_name)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 更新子执行记录
    pub async fn update(
        pool: &SqlitePool,
        input: UpdateBatchSubExecutionInput,
    ) -> Result<BatchSubExecution, AppError> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE batch_sub_execution SET status = COALESCE(?, status), error_code = COALESCE(?, error_code), error_message = COALESCE(?, error_message), result_comment_id = COALESCE(?, result_comment_id), execution_record_id = COALESCE(?, execution_record_id), updated_at = ? WHERE id = ?",
        )
        .bind(input.status.map(|s| s.to_string()))
        .bind(input.error_code)
        .bind(input.error_message)
        .bind(input.result_comment_id)
        .bind(input.execution_record_id)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    /// 根据 ID 查询子执行
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<BatchSubExecution, AppError> {
        sqlx::query_as::<_, BatchSubExecution>(
            "SELECT id, parent_task_id, student_id, student_name, status, error_code, error_message, result_comment_id, execution_record_id, created_at, updated_at FROM batch_sub_execution WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("子执行不存在：{id}")))
    }

    /// 根据父任务查询子执行列表
    pub async fn list_by_parent(
        pool: &SqlitePool,
        input: ListBatchSubExecutionsInput,
    ) -> Result<Vec<BatchSubExecution>, AppError> {
        let mut query = String::from(
            "SELECT id, parent_task_id, student_id, student_name, status, error_code, error_message, result_comment_id, execution_record_id, created_at, updated_at FROM batch_sub_execution WHERE parent_task_id = ?",
        );

        if input.status.is_some() {
            query.push_str(" AND status = ?");
        }

        query.push_str(" ORDER BY created_at");

        let mut sql = sqlx::query_as::<_, BatchSubExecution>(&query).bind(&input.parent_task_id);

        if let Some(status) = input.status {
            sql = sql.bind(status.to_string());
        }

        sql.fetch_all(pool).await.map_err(|e| e.into())
    }

    /// 获取子执行统计
    pub async fn get_statistics(
        pool: &SqlitePool,
        parent_task_id: &str,
    ) -> Result<BatchSubExecutionStatistics, AppError> {
        let row: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT COUNT(*) as total, COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed, COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed, COUNT(CASE WHEN status = 'running' THEN 1 END) as running FROM batch_sub_execution WHERE parent_task_id = ?",
        )
        .bind(parent_task_id)
        .fetch_one(pool)
        .await?;

        Ok(BatchSubExecutionStatistics {
            total: row.0,
            completed: row.1,
            failed: row.2,
            running: row.3,
        })
    }
}

/// 子执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSubExecutionStatistics {
    pub total: i64,
    pub completed: i64,
    pub failed: i64,
    pub running: i64,
}

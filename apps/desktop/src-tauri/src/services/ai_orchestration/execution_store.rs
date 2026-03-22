//! 执行运行时存储服务
//!
//! 提供 execution_session / execution_message / execution_record 三表的统一持久化接口。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::execution::{
    CreateExecutionMessageInput, CreateExecutionRecordInput, CreateExecutionSessionInput,
    ExecutionMessage, ExecutionRecord, ExecutionSession, ExecutionStatus,
    UpdateExecutionRecordInput,
};

/// 执行运行时存储服务
pub struct ExecutionStoreService;

impl ExecutionStoreService {
    pub async fn create_session(
        pool: &SqlitePool,
        input: CreateExecutionSessionInput,
    ) -> Result<ExecutionSession, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO execution_session (id, teacher_id, title, entrypoint, agent_profile_id, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.teacher_id)
        .bind(&input.title)
        .bind(serde_json::to_string(&input.entrypoint).unwrap_or_else(|_| String::from("\"chat\""))
            .trim_matches('"')
            .to_string())
        .bind(&input.agent_profile_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Self::get_session_by_id(pool, &id).await
    }

    pub async fn create_message(
        pool: &SqlitePool,
        input: CreateExecutionMessageInput,
    ) -> Result<ExecutionMessage, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO execution_message (id, session_id, role, content, tool_name, is_deleted, created_at) VALUES (?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(&id)
        .bind(&input.session_id)
        .bind(&input.role)
        .bind(&input.content)
        .bind(&input.tool_name)
        .bind(&now)
        .execute(pool)
        .await?;

        sqlx::query("UPDATE execution_session SET updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&input.session_id)
            .execute(pool)
            .await?;

        Self::get_message_by_id(pool, &id).await
    }

    pub async fn create_record(
        pool: &SqlitePool,
        input: CreateExecutionRecordInput,
    ) -> Result<ExecutionRecord, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO execution_record (id, session_id, execution_message_id, entrypoint, agent_profile_id, model_id, status, reasoning_summary, search_summary_json, tool_calls_summary_json, error_message, metadata_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&input.session_id)
        .bind(&input.execution_message_id)
        .bind(serde_json::to_string(&input.entrypoint).unwrap_or_else(|_| String::from("\"chat\""))
            .trim_matches('"')
            .to_string())
        .bind(&input.agent_profile_id)
        .bind(&input.model_id)
        .bind(serde_json::to_string(&input.status).unwrap_or_else(|_| String::from("\"completed\""))
            .trim_matches('"')
            .to_string())
        .bind(&input.reasoning_summary)
        .bind(&input.search_summary_json)
        .bind(&input.tool_calls_summary_json)
        .bind(input.metadata_json.map(|value| value.to_string()))
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Self::get_record_by_id(pool, &id).await
    }

    pub async fn finalize_success(
        pool: &SqlitePool,
        input: UpdateExecutionRecordInput,
    ) -> Result<ExecutionRecord, AppError> {
        let now = Utc::now().to_rfc3339();
        let status = input.status.unwrap_or(ExecutionStatus::Completed);

        sqlx::query(
            "UPDATE execution_record SET status = ?, reasoning_summary = COALESCE(?, reasoning_summary), search_summary_json = COALESCE(?, search_summary_json), tool_calls_summary_json = COALESCE(?, tool_calls_summary_json), error_message = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(serde_json::to_string(&status).unwrap_or_else(|_| String::from("\"completed\""))
            .trim_matches('"')
            .to_string())
        .bind(&input.reasoning_summary)
        .bind(&input.search_summary_json)
        .bind(&input.tool_calls_summary_json)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        Self::get_record_by_id(pool, &input.id).await
    }

    pub async fn record_failure(
        pool: &SqlitePool,
        input: UpdateExecutionRecordInput,
    ) -> Result<ExecutionRecord, AppError> {
        let now = Utc::now().to_rfc3339();
        let status = input.status.unwrap_or(ExecutionStatus::Failed);

        sqlx::query(
            "UPDATE execution_record SET status = ?, error_message = COALESCE(?, error_message), reasoning_summary = COALESCE(?, reasoning_summary), search_summary_json = COALESCE(?, search_summary_json), tool_calls_summary_json = COALESCE(?, tool_calls_summary_json), updated_at = ? WHERE id = ?",
        )
        .bind(serde_json::to_string(&status).unwrap_or_else(|_| String::from("\"failed\""))
            .trim_matches('"')
            .to_string())
        .bind(&input.error_message)
        .bind(&input.reasoning_summary)
        .bind(&input.search_summary_json)
        .bind(&input.tool_calls_summary_json)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        Self::get_record_by_id(pool, &input.id).await
    }

    pub async fn get_session_by_id(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<ExecutionSession, AppError> {
        sqlx::query_as::<_, ExecutionSession>(
            "SELECT id, teacher_id, title, entrypoint, agent_profile_id, is_deleted, created_at, updated_at FROM execution_session WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("执行会话不存在：{id}")))
    }

    pub async fn get_message_by_id(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<ExecutionMessage, AppError> {
        sqlx::query_as::<_, ExecutionMessage>(
            "SELECT id, session_id, role, content, tool_name, is_deleted, created_at FROM execution_message WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("执行消息不存在：{id}")))
    }

    pub async fn get_record_by_id(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<ExecutionRecord, AppError> {
        sqlx::query_as::<_, ExecutionRecord>(
            "SELECT id, session_id, execution_message_id, entrypoint, agent_profile_id, model_id, status, reasoning_summary, search_summary_json, tool_calls_summary_json, error_message, metadata_json, created_at, updated_at FROM execution_record WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("执行记录不存在：{id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::ConnectOptions;
    use std::str::FromStr;

    async fn test_pool() -> SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true)
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query("INSERT INTO teacher_profile (id, name, stage, subject, created_at, updated_at, is_deleted) VALUES (?, ?, ?, ?, ?, ?, 0)")
            .bind("teacher-1")
            .bind("测试教师")
            .bind("primary")
            .bind("数学")
            .bind("2026-03-22T00:00:00Z")
            .bind("2026-03-22T00:00:00Z")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    /// 验证可创建执行会话
    #[tokio::test]
    async fn test_create_execution_session() {
        let pool = test_pool().await;
        let session = ExecutionStoreService::create_session(
            &pool,
            CreateExecutionSessionInput {
                teacher_id: String::from("teacher-1"),
                title: Some(String::from("测试会话")),
                entrypoint: crate::models::execution::ExecutionEntrypoint::Chat,
                agent_profile_id: String::from("chat.homeroom"),
            },
        )
        .await
        .unwrap();

        assert_eq!(session.teacher_id, "teacher-1");
        assert_eq!(session.entrypoint, "chat");
    }

    /// 验证可创建执行消息
    #[tokio::test]
    async fn test_create_execution_message() {
        let pool = test_pool().await;
        let session = ExecutionStoreService::create_session(
            &pool,
            CreateExecutionSessionInput {
                teacher_id: String::from("teacher-1"),
                title: None,
                entrypoint: crate::models::execution::ExecutionEntrypoint::Chat,
                agent_profile_id: String::from("chat.homeroom"),
            },
        )
        .await
        .unwrap();

        let message = ExecutionStoreService::create_message(
            &pool,
            CreateExecutionMessageInput {
                session_id: session.id,
                role: String::from("assistant"),
                content: String::from("你好"),
                tool_name: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(message.role, "assistant");
        assert_eq!(message.content, "你好");
    }

    /// 验证可完成成功记录
    #[tokio::test]
    async fn test_finalize_success() {
        let pool = test_pool().await;
        let session = ExecutionStoreService::create_session(
            &pool,
            CreateExecutionSessionInput {
                teacher_id: String::from("teacher-1"),
                title: None,
                entrypoint: crate::models::execution::ExecutionEntrypoint::Chat,
                agent_profile_id: String::from("chat.homeroom"),
            },
        )
        .await
        .unwrap();
        let message = ExecutionStoreService::create_message(
            &pool,
            CreateExecutionMessageInput {
                session_id: session.id.clone(),
                role: String::from("assistant"),
                content: String::from("草稿"),
                tool_name: None,
            },
        )
        .await
        .unwrap();
        let record = ExecutionStoreService::create_record(
            &pool,
            CreateExecutionRecordInput {
                session_id: session.id,
                execution_message_id: message.id,
                entrypoint: crate::models::execution::ExecutionEntrypoint::Chat,
                agent_profile_id: String::from("chat.homeroom"),
                model_id: String::from("gpt-4o-mini"),
                status: ExecutionStatus::Failed,
                reasoning_summary: None,
                search_summary_json: None,
                tool_calls_summary_json: None,
                metadata_json: None,
            },
        )
        .await
        .unwrap();

        let updated = ExecutionStoreService::finalize_success(
            &pool,
            UpdateExecutionRecordInput {
                id: record.id,
                status: Some(ExecutionStatus::Completed),
                reasoning_summary: Some(String::from("推理完成")),
                search_summary_json: Some(String::from("{\"count\":1}")),
                tool_calls_summary_json: Some(String::from("[]")),
                error_message: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.status, "completed");
        assert_eq!(updated.reasoning_summary.as_deref(), Some("推理完成"));
        assert!(updated.error_message.is_none());
    }

    /// 验证可记录失败摘要
    #[tokio::test]
    async fn test_record_failure_summary() {
        let pool = test_pool().await;
        let session = ExecutionStoreService::create_session(
            &pool,
            CreateExecutionSessionInput {
                teacher_id: String::from("teacher-1"),
                title: None,
                entrypoint: crate::models::execution::ExecutionEntrypoint::Chat,
                agent_profile_id: String::from("chat.homeroom"),
            },
        )
        .await
        .unwrap();
        let message = ExecutionStoreService::create_message(
            &pool,
            CreateExecutionMessageInput {
                session_id: session.id.clone(),
                role: String::from("assistant"),
                content: String::from("草稿"),
                tool_name: None,
            },
        )
        .await
        .unwrap();
        let record = ExecutionStoreService::create_record(
            &pool,
            CreateExecutionRecordInput {
                session_id: session.id,
                execution_message_id: message.id,
                entrypoint: crate::models::execution::ExecutionEntrypoint::Chat,
                agent_profile_id: String::from("chat.homeroom"),
                model_id: String::from("gpt-4o-mini"),
                status: ExecutionStatus::Completed,
                reasoning_summary: None,
                search_summary_json: None,
                tool_calls_summary_json: None,
                metadata_json: None,
            },
        )
        .await
        .unwrap();

        let updated = ExecutionStoreService::record_failure(
            &pool,
            UpdateExecutionRecordInput {
                id: record.id,
                status: Some(ExecutionStatus::Failed),
                reasoning_summary: Some(String::from("失败前已检索")),
                search_summary_json: None,
                tool_calls_summary_json: None,
                error_message: Some(String::from("模型超时")),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.status, "failed");
        assert_eq!(updated.error_message.as_deref(), Some("模型超时"));
    }
}

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::observation_note::{
    CreateObservationNoteInput, ObservationNote, UpdateObservationNoteInput,
};
use crate::services::audit::AuditService;

pub struct ObservationNoteService;

impl ObservationNoteService {
    pub async fn list_student_observations(
        pool: &SqlitePool,
        student_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ObservationNote>, AppError> {
        Self::validate_student_exists(pool, student_id).await?;

        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        if limit <= 0 {
            return Err(AppError::InvalidInput(String::from(
                "limit 必须是大于 0 的整数",
            )));
        }

        if offset < 0 {
            return Err(AppError::InvalidInput(String::from("offset 不能为负数")));
        }

        let notes = sqlx::query_as::<_, ObservationNote>(
            "SELECT id, student_id, content, source, created_at, is_deleted, updated_at FROM observation_note WHERE student_id = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(student_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(notes)
    }

    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<ObservationNote, AppError> {
        let note = sqlx::query_as::<_, ObservationNote>(
            "SELECT id, student_id, content, source, created_at, is_deleted, updated_at FROM observation_note WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("观察记录不存在：{id}")))?;

        Ok(note)
    }

    pub async fn create(
        pool: &SqlitePool,
        input: CreateObservationNoteInput,
    ) -> Result<ObservationNote, AppError> {
        Self::validate_student_exists(pool, &input.student_id).await?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO observation_note (id, student_id, content, source, created_at, is_deleted, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(&id)
        .bind(&input.student_id)
        .bind(&input.content)
        .bind(&input.source)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_observation_note",
            "observation_note",
            Some(&id),
            "low",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    pub async fn update(
        pool: &SqlitePool,
        input: UpdateObservationNoteInput,
    ) -> Result<ObservationNote, AppError> {
        let has_updates = input.content.is_some() || input.source.is_some();
        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        Self::get_by_id(pool, &input.id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE observation_note SET content = COALESCE(?, content), source = COALESCE(?, source), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.content)
        .bind(&input.source)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("观察记录不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_observation_note",
            "observation_note",
            Some(&input.id),
            "low",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE observation_note SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("观察记录不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_observation_note",
            "observation_note",
            Some(id),
            "medium",
            false,
        )
        .await?;

        Ok(())
    }

    async fn validate_student_exists(pool: &SqlitePool, student_id: &str) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM student WHERE id = ? AND is_deleted = 0",
        )
        .bind(student_id)
        .fetch_one(pool)
        .await?;

        if exists == 0 {
            return Err(AppError::InvalidInput(format!(
                "学生不存在或已删除：{student_id}"
            )));
        }

        Ok(())
    }
}

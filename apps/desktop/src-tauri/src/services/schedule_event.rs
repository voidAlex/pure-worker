use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::schedule_event::{
    CreateScheduleEventInput, ScheduleEvent, UpdateScheduleEventInput,
};
use crate::services::audit::AuditService;

pub struct ScheduleEventService;

impl ScheduleEventService {
    pub async fn list_by_class(
        pool: &SqlitePool,
        class_id: &str,
    ) -> Result<Vec<ScheduleEvent>, AppError> {
        Self::validate_classroom_exists(pool, class_id).await?;

        let events = sqlx::query_as::<_, ScheduleEvent>(
            "SELECT id, class_id, title, start_at, end_at, linked_file_id, is_deleted, created_at FROM schedule_event WHERE class_id = ? AND is_deleted = 0 ORDER BY start_at ASC, created_at DESC",
        )
        .bind(class_id)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<ScheduleEvent, AppError> {
        let event = sqlx::query_as::<_, ScheduleEvent>(
            "SELECT id, class_id, title, start_at, end_at, linked_file_id, is_deleted, created_at FROM schedule_event WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("日程事件不存在：{id}")))?;

        Ok(event)
    }

    pub async fn create(
        pool: &SqlitePool,
        input: CreateScheduleEventInput,
    ) -> Result<ScheduleEvent, AppError> {
        Self::validate_classroom_exists(pool, &input.class_id).await?;
        Self::validate_time_range(&input.start_at, input.end_at.as_deref())?;
        Self::validate_no_conflict(
            pool,
            &input.class_id,
            &input.start_at,
            input.end_at.as_deref(),
            None,
        )
        .await?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO schedule_event (id, class_id, title, start_at, end_at, linked_file_id, is_deleted, created_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(&id)
        .bind(&input.class_id)
        .bind(&input.title)
        .bind(&input.start_at)
        .bind(&input.end_at)
        .bind(&input.linked_file_id)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_schedule_event",
            "schedule_event",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    pub async fn update(
        pool: &SqlitePool,
        input: UpdateScheduleEventInput,
    ) -> Result<ScheduleEvent, AppError> {
        let has_updates = input.title.is_some()
            || input.start_at.is_some()
            || input.end_at.is_some()
            || input.linked_file_id.is_some();
        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        let existing = Self::get_by_id(pool, &input.id).await?;
        let target_start = input
            .start_at
            .clone()
            .unwrap_or_else(|| existing.start_at.clone());
        let target_end = input.end_at.clone().or(existing.end_at.clone());

        Self::validate_time_range(&target_start, target_end.as_deref())?;
        Self::validate_no_conflict(
            pool,
            &existing.class_id,
            &target_start,
            target_end.as_deref(),
            Some(&input.id),
        )
        .await?;

        let result = sqlx::query(
            "UPDATE schedule_event SET title = COALESCE(?, title), start_at = COALESCE(?, start_at), end_at = COALESCE(?, end_at), linked_file_id = COALESCE(?, linked_file_id) WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.title)
        .bind(&input.start_at)
        .bind(&input.end_at)
        .bind(&input.linked_file_id)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("日程事件不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_schedule_event",
            "schedule_event",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let result =
            sqlx::query("UPDATE schedule_event SET is_deleted = 1 WHERE id = ? AND is_deleted = 0")
                .bind(id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("日程事件不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_schedule_event",
            "schedule_event",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    fn validate_time_range(start_at: &str, end_at: Option<&str>) -> Result<(), AppError> {
        if let Some(end_at) = end_at {
            if end_at <= start_at {
                return Err(AppError::InvalidInput(String::from(
                    "结束时间必须晚于开始时间",
                )));
            }
        }

        Ok(())
    }

    async fn validate_classroom_exists(pool: &SqlitePool, class_id: &str) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM classroom WHERE id = ? AND is_deleted = 0",
        )
        .bind(class_id)
        .fetch_one(pool)
        .await?;

        if exists == 0 {
            return Err(AppError::InvalidInput(format!(
                "班级不存在或已删除：{class_id}"
            )));
        }

        Ok(())
    }

    async fn validate_no_conflict(
        pool: &SqlitePool,
        class_id: &str,
        start_at: &str,
        end_at: Option<&str>,
        exclude_id: Option<&str>,
    ) -> Result<(), AppError> {
        let new_end = end_at.unwrap_or(start_at);

        let conflict_count = if let Some(exclude_id) = exclude_id {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM schedule_event WHERE class_id = ? AND is_deleted = 0 AND id <> ? AND start_at < ? AND COALESCE(end_at, start_at) > ?",
            )
            .bind(class_id)
            .bind(exclude_id)
            .bind(new_end)
            .bind(start_at)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM schedule_event WHERE class_id = ? AND is_deleted = 0 AND start_at < ? AND COALESCE(end_at, start_at) > ?",
            )
            .bind(class_id)
            .bind(new_end)
            .bind(start_at)
            .fetch_one(pool)
            .await?
        };

        if conflict_count > 0 {
            return Err(AppError::InvalidInput(String::from(
                "当前班级在该时间段存在冲突事件",
            )));
        }

        Ok(())
    }
}

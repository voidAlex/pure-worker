use chrono::Utc;
use sqlx::QueryBuilder;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::lesson_record::{
    CreateLessonRecordInput, LessonRecord, LessonSummary, ListLessonRecordsInput,
    UpdateLessonRecordInput,
};
use crate::services::audit::AuditService;

pub struct LessonRecordService;

impl LessonRecordService {
    pub async fn create_lesson_record(
        pool: &SqlitePool,
        input: CreateLessonRecordInput,
    ) -> Result<LessonRecord, AppError> {
        Self::validate_classroom_exists(pool, &input.class_id).await?;

        if let Some(ref schedule_event_id) = input.schedule_event_id {
            Self::validate_schedule_event_exists(pool, schedule_event_id).await?;
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let status = input.status.unwrap_or_else(|| "planned".to_string());

        sqlx::query(
            "INSERT INTO lesson_record (id, class_id, schedule_event_id, subject, lesson_date, lesson_index, topic, teaching_goal, homework_summary, teacher_note, status, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.class_id)
        .bind(&input.schedule_event_id)
        .bind(&input.subject)
        .bind(&input.lesson_date)
        .bind(input.lesson_index)
        .bind(&input.topic)
        .bind(&input.teaching_goal)
        .bind(&input.homework_summary)
        .bind(&input.teacher_note)
        .bind(&status)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_lesson_record",
            "lesson_record",
            Some(&id),
            "low",
            false,
        )
        .await?;

        Self::get_lesson_record(pool, &id).await
    }

    pub async fn get_lesson_record(pool: &SqlitePool, id: &str) -> Result<LessonRecord, AppError> {
        let record = sqlx::query_as::<_, LessonRecord>(
            "SELECT id, class_id, schedule_event_id, subject, lesson_date, lesson_index, topic, teaching_goal, homework_summary, teacher_note, status, is_deleted, created_at, updated_at FROM lesson_record WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("课程记录不存在：{id}")))?;

        Ok(record)
    }

    pub async fn update_lesson_record(
        pool: &SqlitePool,
        input: UpdateLessonRecordInput,
    ) -> Result<LessonRecord, AppError> {
        let has_updates = input.subject.is_some()
            || input.lesson_date.is_some()
            || input.lesson_index.is_some()
            || input.topic.is_some()
            || input.teaching_goal.is_some()
            || input.homework_summary.is_some()
            || input.teacher_note.is_some()
            || input.status.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        Self::get_lesson_record(pool, &input.id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE lesson_record SET subject = COALESCE(?, subject), lesson_date = COALESCE(?, lesson_date), lesson_index = COALESCE(?, lesson_index), topic = COALESCE(?, topic), teaching_goal = COALESCE(?, teaching_goal), homework_summary = COALESCE(?, homework_summary), teacher_note = COALESCE(?, teacher_note), status = COALESCE(?, status), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.subject)
        .bind(&input.lesson_date)
        .bind(input.lesson_index)
        .bind(&input.topic)
        .bind(&input.teaching_goal)
        .bind(&input.homework_summary)
        .bind(&input.teacher_note)
        .bind(&input.status)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("课程记录不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_lesson_record",
            "lesson_record",
            Some(&input.id),
            "low",
            false,
        )
        .await?;

        Self::get_lesson_record(pool, &input.id).await
    }

    pub async fn delete_lesson_record(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result =
            sqlx::query("UPDATE lesson_record SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0")
                .bind(&now)
                .bind(id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("课程记录不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_lesson_record",
            "lesson_record",
            Some(id),
            "medium",
            false,
        )
        .await?;

        Ok(())
    }

    pub async fn list_lesson_records(
        pool: &SqlitePool,
        input: ListLessonRecordsInput,
    ) -> Result<Vec<LessonRecord>, AppError> {
        let mut query = QueryBuilder::new(
            "SELECT id, class_id, schedule_event_id, subject, lesson_date, lesson_index, topic, teaching_goal, homework_summary, teacher_note, status, is_deleted, created_at, updated_at FROM lesson_record WHERE is_deleted = 0",
        );

        if let Some(class_id) = &input.class_id {
            query.push(" AND class_id = ").push_bind(class_id);
        }

        if let Some(from_date) = &input.from_date {
            query.push(" AND lesson_date >= ").push_bind(from_date);
        }

        if let Some(to_date) = &input.to_date {
            query.push(" AND lesson_date <= ").push_bind(to_date);
        }

        if let Some(status) = &input.status {
            query.push(" AND status = ").push_bind(status);
        }

        query.push(" ORDER BY lesson_date DESC, lesson_index ASC, created_at DESC");

        let records = query
            .build_query_as::<LessonRecord>()
            .fetch_all(pool)
            .await?;

        Ok(records)
    }

    pub async fn get_lesson_summary(
        pool: &SqlitePool,
        lesson_record_id: &str,
    ) -> Result<LessonSummary, AppError> {
        let lesson = Self::get_lesson_record(pool, lesson_record_id).await?;

        let observation_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM observation_note WHERE lesson_record_id = ? AND is_deleted = 0",
        )
        .bind(lesson_record_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let score_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM score_record WHERE lesson_record_id = ? AND is_deleted = 0",
        )
        .bind(lesson_record_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let assignment_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM assignment_asset WHERE lesson_record_id = ? AND is_deleted = 0",
        )
        .bind(lesson_record_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let communication_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM parent_communication WHERE lesson_record_id = ? AND is_deleted = 0",
        )
        .bind(lesson_record_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        Ok(LessonSummary {
            lesson_record_id: lesson.id,
            class_id: lesson.class_id,
            subject: lesson.subject,
            lesson_date: lesson.lesson_date,
            topic: lesson.topic,
            status: lesson.status,
            observation_count,
            score_count,
            assignment_count,
            communication_count,
        })
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

    async fn validate_schedule_event_exists(
        pool: &SqlitePool,
        schedule_event_id: &str,
    ) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM schedule_event WHERE id = ? AND is_deleted = 0",
        )
        .bind(schedule_event_id)
        .fetch_one(pool)
        .await?;

        if exists == 0 {
            return Err(AppError::InvalidInput(format!(
                "日程事件不存在或已删除：{schedule_event_id}"
            )));
        }

        Ok(())
    }
}

use chrono::Utc;
use sqlx::QueryBuilder;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::score_record::{CreateScoreRecordInput, ScoreRecord, UpdateScoreRecordInput};
use crate::services::audit::AuditService;

pub struct ScoreRecordService;

impl ScoreRecordService {
    pub async fn list_student_scores(
        pool: &SqlitePool,
        student_id: &str,
        subject: Option<&str>,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> Result<Vec<ScoreRecord>, AppError> {
        Self::validate_student_exists(pool, student_id).await?;

        let mut query = QueryBuilder::new(
            "SELECT id, student_id, exam_name, subject, score, full_score, rank_in_class, exam_date, is_deleted, updated_at FROM score_record WHERE is_deleted = 0 AND student_id = ",
        );
        query.push_bind(student_id);

        if let Some(subject) = subject {
            query.push(" AND subject = ").push_bind(subject);
        }

        if let Some(from_date) = from_date {
            query.push(" AND exam_date >= ").push_bind(from_date);
        }

        if let Some(to_date) = to_date {
            query.push(" AND exam_date <= ").push_bind(to_date);
        }

        query.push(" ORDER BY exam_date DESC, updated_at DESC");

        let records = query
            .build_query_as::<ScoreRecord>()
            .fetch_all(pool)
            .await?;
        Ok(records)
    }

    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<ScoreRecord, AppError> {
        let record = sqlx::query_as::<_, ScoreRecord>(
            "SELECT id, student_id, exam_name, subject, score, full_score, rank_in_class, exam_date, is_deleted, updated_at FROM score_record WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("成绩记录不存在：{id}")))?;

        Ok(record)
    }

    pub async fn create(
        pool: &SqlitePool,
        input: CreateScoreRecordInput,
    ) -> Result<ScoreRecord, AppError> {
        Self::validate_student_exists(pool, &input.student_id).await?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO score_record (id, student_id, exam_name, subject, score, full_score, rank_in_class, exam_date, is_deleted, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(&id)
        .bind(&input.student_id)
        .bind(&input.exam_name)
        .bind(&input.subject)
        .bind(input.score)
        .bind(input.full_score)
        .bind(input.rank_in_class)
        .bind(&input.exam_date)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_score_record",
            "score_record",
            Some(&id),
            "low",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    pub async fn update(
        pool: &SqlitePool,
        input: UpdateScoreRecordInput,
    ) -> Result<ScoreRecord, AppError> {
        let has_updates = input.exam_name.is_some()
            || input.subject.is_some()
            || input.score.is_some()
            || input.full_score.is_some()
            || input.rank_in_class.is_some()
            || input.exam_date.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        Self::get_by_id(pool, &input.id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE score_record SET exam_name = COALESCE(?, exam_name), subject = COALESCE(?, subject), score = COALESCE(?, score), full_score = COALESCE(?, full_score), rank_in_class = COALESCE(?, rank_in_class), exam_date = COALESCE(?, exam_date), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.exam_name)
        .bind(&input.subject)
        .bind(input.score)
        .bind(input.full_score)
        .bind(input.rank_in_class)
        .bind(&input.exam_date)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("成绩记录不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_score_record",
            "score_record",
            Some(&input.id),
            "low",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result =
            sqlx::query("UPDATE score_record SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0")
                .bind(&now)
                .bind(id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("成绩记录不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_score_record",
            "score_record",
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

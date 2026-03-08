use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::student_tag::{AddStudentTagInput, StudentTag, UpdateStudentTagInput};
use crate::services::audit::AuditService;

pub struct StudentTagService;

impl StudentTagService {
    pub async fn list_by_student(
        pool: &SqlitePool,
        student_id: &str,
    ) -> Result<Vec<StudentTag>, AppError> {
        Self::validate_student_exists(pool, student_id).await?;

        let tags = sqlx::query_as::<_, StudentTag>(
            "SELECT id, student_id, tag_name, is_deleted, created_at FROM student_tag WHERE student_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(student_id)
        .fetch_all(pool)
        .await?;

        Ok(tags)
    }

    pub async fn add(pool: &SqlitePool, input: AddStudentTagInput) -> Result<StudentTag, AppError> {
        let tag_name = input.tag_name.trim();
        if tag_name.is_empty() {
            return Err(AppError::InvalidInput(String::from("标签名称不能为空")));
        }

        Self::validate_student_exists(pool, &input.student_id).await?;
        Self::validate_unique_tag(pool, &input.student_id, tag_name, None).await?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO student_tag (id, student_id, tag_name, is_deleted, created_at) VALUES (?, ?, ?, 0, ?)",
        )
        .bind(&id)
        .bind(&input.student_id)
        .bind(tag_name)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "add_student_tag",
            "student_tag",
            Some(&id),
            "low",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 更新学生标签名称
    pub async fn update(
        pool: &SqlitePool,
        input: UpdateStudentTagInput,
    ) -> Result<StudentTag, AppError> {
        let tag_name = input.tag_name.trim();
        if tag_name.is_empty() {
            return Err(AppError::InvalidInput(String::from("标签名称不能为空")));
        }

        let existing = Self::get_by_id(pool, &input.id).await?;
        Self::validate_unique_tag(pool, &existing.student_id, tag_name, Some(&input.id)).await?;

        sqlx::query("UPDATE student_tag SET tag_name = ? WHERE id = ? AND is_deleted = 0")
            .bind(tag_name)
            .bind(&input.id)
            .execute(pool)
            .await?;

        AuditService::log(
            pool,
            "system",
            "update_student_tag",
            "student_tag",
            Some(&input.id),
            "low",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    pub async fn remove(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let result =
            sqlx::query("UPDATE student_tag SET is_deleted = 1 WHERE id = ? AND is_deleted = 0")
                .bind(id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("学生标签不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "remove_student_tag",
            "student_tag",
            Some(id),
            "medium",
            false,
        )
        .await?;

        Ok(())
    }

    async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<StudentTag, AppError> {
        let tag = sqlx::query_as::<_, StudentTag>(
            "SELECT id, student_id, tag_name, is_deleted, created_at FROM student_tag WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("学生标签不存在：{id}")))?;

        Ok(tag)
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

    async fn validate_unique_tag(
        pool: &SqlitePool,
        student_id: &str,
        tag_name: &str,
        exclude_id: Option<&str>,
    ) -> Result<(), AppError> {
        let exists = if let Some(exclude_id) = exclude_id {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM student_tag WHERE student_id = ? AND tag_name = ? AND is_deleted = 0 AND id <> ?",
            )
            .bind(student_id)
            .bind(tag_name)
            .bind(exclude_id)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM student_tag WHERE student_id = ? AND tag_name = ? AND is_deleted = 0",
            )
            .bind(student_id)
            .bind(tag_name)
            .fetch_one(pool)
            .await?
        };

        if exists > 0 {
            return Err(AppError::InvalidInput(format!(
                "标签已存在：student_id={student_id}, tag_name={tag_name}"
            )));
        }

        Ok(())
    }
}

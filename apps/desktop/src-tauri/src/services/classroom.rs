use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::classroom::{Classroom, CreateClassroomInput, UpdateClassroomInput};
use crate::services::audit::AuditService;

pub struct ClassroomService;

impl ClassroomService {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Classroom>, AppError> {
        let classrooms = sqlx::query_as::<_, Classroom>(
            "SELECT id, grade, class_name, subject, teacher_id, is_deleted, created_at, updated_at FROM classroom WHERE is_deleted = 0 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(classrooms)
    }

    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Classroom, AppError> {
        let classroom = sqlx::query_as::<_, Classroom>(
            "SELECT id, grade, class_name, subject, teacher_id, is_deleted, created_at, updated_at FROM classroom WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("班级不存在：{id}")))?;

        Ok(classroom)
    }

    pub async fn create(
        pool: &SqlitePool,
        input: CreateClassroomInput,
    ) -> Result<Classroom, AppError> {
        // TODO: 教师档案管理功能完成后恢复校验

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO classroom (id, grade, class_name, subject, teacher_id, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.grade)
        .bind(&input.class_name)
        .bind(&input.subject)
        .bind(&input.teacher_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_classroom",
            "classroom",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    pub async fn update(
        pool: &SqlitePool,
        input: UpdateClassroomInput,
    ) -> Result<Classroom, AppError> {
        let has_updates = input.grade.is_some()
            || input.class_name.is_some()
            || input.subject.is_some()
            || input.teacher_id.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        Self::get_by_id(pool, &input.id).await?;

        // TODO: 教师档案管理功能完成后恢复校验
        // if let Some(teacher_id) = input.teacher_id.as_deref() {
        //     Self::validate_teacher_exists(pool, teacher_id).await?;
        // }

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE classroom SET grade = COALESCE(?, grade), class_name = COALESCE(?, class_name), subject = COALESCE(?, subject), teacher_id = COALESCE(?, teacher_id), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.grade)
        .bind(&input.class_name)
        .bind(&input.subject)
        .bind(&input.teacher_id)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("班级不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_classroom",
            "classroom",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE classroom SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("班级不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_classroom",
            "classroom",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 校验教师是否存在（教师档案管理功能完成后启用）。
    #[allow(dead_code)]
    async fn validate_teacher_exists(pool: &SqlitePool, teacher_id: &str) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM teacher_profile WHERE id = ? AND is_deleted = 0",
        )
        .bind(teacher_id)
        .fetch_one(pool)
        .await?;

        if exists == 0 {
            return Err(AppError::InvalidInput(format!(
                "教师不存在或已删除：{teacher_id}"
            )));
        }

        Ok(())
    }
}

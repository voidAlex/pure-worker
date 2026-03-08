use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::parent_communication::{
    CreateParentCommunicationInput, ParentCommunication, UpdateParentCommunicationInput,
};
use crate::services::audit::AuditService;

pub struct ParentCommunicationService;

impl ParentCommunicationService {
    pub async fn list_by_student(
        pool: &SqlitePool,
        student_id: &str,
    ) -> Result<Vec<ParentCommunication>, AppError> {
        Self::validate_student_exists(pool, student_id).await?;

        let items = sqlx::query_as::<_, ParentCommunication>(
            "SELECT id, student_id, draft, adopted_text, status, evidence_json, created_at, is_deleted, updated_at FROM parent_communication WHERE student_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(student_id)
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<ParentCommunication, AppError> {
        let item = sqlx::query_as::<_, ParentCommunication>(
            "SELECT id, student_id, draft, adopted_text, status, evidence_json, created_at, is_deleted, updated_at FROM parent_communication WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("家校沟通记录不存在：{id}")))?;

        Ok(item)
    }

    pub async fn create(
        pool: &SqlitePool,
        input: CreateParentCommunicationInput,
    ) -> Result<ParentCommunication, AppError> {
        Self::validate_student_exists(pool, &input.student_id).await?;

        let status = input.status.unwrap_or_else(|| String::from("draft"));
        Self::validate_status(&status)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO parent_communication (id, student_id, draft, adopted_text, status, evidence_json, created_at, is_deleted, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(&id)
        .bind(&input.student_id)
        .bind(&input.draft)
        .bind(&input.adopted_text)
        .bind(&status)
        .bind(&input.evidence_json)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_parent_communication",
            "parent_communication",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    pub async fn update(
        pool: &SqlitePool,
        input: UpdateParentCommunicationInput,
    ) -> Result<ParentCommunication, AppError> {
        let has_updates = input.draft.is_some()
            || input.adopted_text.is_some()
            || input.status.is_some()
            || input.evidence_json.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        if let Some(status) = input.status.as_deref() {
            Self::validate_status(status)?;
        }

        Self::get_by_id(pool, &input.id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE parent_communication SET draft = COALESCE(?, draft), adopted_text = COALESCE(?, adopted_text), status = COALESCE(?, status), evidence_json = COALESCE(?, evidence_json), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.draft)
        .bind(&input.adopted_text)
        .bind(&input.status)
        .bind(&input.evidence_json)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "家校沟通记录不存在：{}",
                input.id
            )));
        }

        AuditService::log(
            pool,
            "system",
            "update_parent_communication",
            "parent_communication",
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
            "UPDATE parent_communication SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("家校沟通记录不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_parent_communication",
            "parent_communication",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    fn validate_status(status: &str) -> Result<(), AppError> {
        if status == "draft" || status == "adopted" || status == "rejected" {
            return Ok(());
        }

        Err(AppError::InvalidInput(format!(
            "status 非法：{status}，仅支持 draft/adopted/rejected"
        )))
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

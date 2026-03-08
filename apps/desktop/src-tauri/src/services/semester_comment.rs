//! 学期评语服务模块
//!
//! 提供学期评语的列表查询、创建、更新、删除与批量采纳能力

use chrono::Utc;
use sqlx::QueryBuilder;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::semester_comment::{
    CreateSemesterCommentInput, ListSemesterCommentsInput, SemesterComment,
    UpdateSemesterCommentInput,
};
use crate::services::audit::AuditService;

/// 学期评语服务
pub struct SemesterCommentService;

impl SemesterCommentService {
    /// 列表查询学期评语，支持 student_id/term/task_id 组合过滤
    pub async fn list(
        pool: &SqlitePool,
        input: ListSemesterCommentsInput,
    ) -> Result<Vec<SemesterComment>, AppError> {
        if let Some(student_id) = input.student_id.as_deref() {
            Self::validate_student_exists(pool, student_id).await?;
        }

        let mut query = QueryBuilder::new(
            "SELECT id, student_id, task_id, term, draft, adopted_text, status, evidence_json, evidence_count, is_deleted, created_at, updated_at FROM semester_comment WHERE is_deleted = 0",
        );

        if let Some(student_id) = input.student_id.as_deref() {
            query.push(" AND student_id = ").push_bind(student_id);
        }

        if let Some(term) = input.term.as_deref() {
            query.push(" AND term = ").push_bind(term);
        }

        if let Some(task_id) = input.task_id.as_deref() {
            query.push(" AND task_id = ").push_bind(task_id);
        }

        query.push(" ORDER BY created_at DESC");

        let items = query
            .build_query_as::<SemesterComment>()
            .fetch_all(pool)
            .await?;

        Ok(items)
    }

    /// 按 ID 获取学期评语
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<SemesterComment, AppError> {
        let item = sqlx::query_as::<_, SemesterComment>(
            "SELECT id, student_id, task_id, term, draft, adopted_text, status, evidence_json, evidence_count, is_deleted, created_at, updated_at FROM semester_comment WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("学期评语不存在：{id}")))?;

        Ok(item)
    }

    /// 创建学期评语
    pub async fn create(
        pool: &SqlitePool,
        input: CreateSemesterCommentInput,
    ) -> Result<SemesterComment, AppError> {
        Self::validate_student_exists(pool, &input.student_id).await?;

        let status = input.status.unwrap_or_else(|| String::from("draft"));
        Self::validate_status(&status)?;

        let evidence_count = input.evidence_count.unwrap_or(0);
        if evidence_count < 0 {
            return Err(AppError::InvalidInput(String::from(
                "evidence_count 不能为负数",
            )));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO semester_comment (id, student_id, task_id, term, draft, adopted_text, status, evidence_json, evidence_count, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.student_id)
        .bind(&input.task_id)
        .bind(&input.term)
        .bind(&input.draft)
        .bind(&input.adopted_text)
        .bind(&status)
        .bind(&input.evidence_json)
        .bind(evidence_count)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_semester_comment",
            "semester_comment",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 更新学期评语
    pub async fn update(
        pool: &SqlitePool,
        input: UpdateSemesterCommentInput,
    ) -> Result<SemesterComment, AppError> {
        let has_updates = input.draft.is_some()
            || input.adopted_text.is_some()
            || input.status.is_some()
            || input.evidence_json.is_some()
            || input.evidence_count.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        if let Some(status) = input.status.as_deref() {
            Self::validate_status(status)?;
        }

        if let Some(evidence_count) = input.evidence_count {
            if evidence_count < 0 {
                return Err(AppError::InvalidInput(String::from(
                    "evidence_count 不能为负数",
                )));
            }
        }

        Self::get_by_id(pool, &input.id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE semester_comment SET draft = COALESCE(?, draft), adopted_text = COALESCE(?, adopted_text), status = COALESCE(?, status), evidence_json = COALESCE(?, evidence_json), evidence_count = COALESCE(?, evidence_count), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.draft)
        .bind(&input.adopted_text)
        .bind(&input.status)
        .bind(&input.evidence_json)
        .bind(input.evidence_count)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("学期评语不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_semester_comment",
            "semester_comment",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    /// 软删除学期评语
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE semester_comment SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("学期评语不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_semester_comment",
            "semester_comment",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 批量采纳任务下所有 draft 状态的评语
    pub async fn batch_adopt(
        pool: &SqlitePool,
        task_id: &str,
    ) -> Result<Vec<SemesterComment>, AppError> {
        let ids = sqlx::query_scalar::<_, String>(
            "SELECT id FROM semester_comment WHERE task_id = ? AND status = 'draft' AND is_deleted = 0",
        )
        .bind(task_id)
        .fetch_all(pool)
        .await?;

        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE semester_comment SET adopted_text = draft, status = 'adopted', updated_at = ? WHERE task_id = ? AND status = 'draft' AND is_deleted = 0",
        )
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(Vec::new());
        }

        AuditService::log(
            pool,
            "system",
            "batch_adopt_semester_comments",
            "semester_comment",
            Some(task_id),
            "medium",
            false,
        )
        .await?;

        let mut query = QueryBuilder::new(
            "SELECT id, student_id, task_id, term, draft, adopted_text, status, evidence_json, evidence_count, is_deleted, created_at, updated_at FROM semester_comment WHERE is_deleted = 0 AND id IN (",
        );

        {
            let mut separated = query.separated(", ");
            for id in &ids {
                separated.push_bind(id);
            }
        }

        query.push(") ORDER BY created_at DESC");

        let items = query
            .build_query_as::<SemesterComment>()
            .fetch_all(pool)
            .await?;

        Ok(items)
    }

    /// 校验状态字段是否合法
    fn validate_status(status: &str) -> Result<(), AppError> {
        if status == "draft" || status == "adopted" || status == "rejected" {
            return Ok(());
        }

        Err(AppError::InvalidInput(format!(
            "status 非法：{status}，仅支持 draft/adopted/rejected"
        )))
    }

    /// 校验学生是否存在且未删除
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

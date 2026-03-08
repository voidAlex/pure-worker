//! 活动公告服务层
//!
//! 提供活动公告的列表、查询、创建、更新、删除及输入校验能力。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::activity_announcement::{
    ActivityAnnouncement, CreateActivityAnnouncementInput, ListActivityAnnouncementsInput,
    UpdateActivityAnnouncementInput,
};
use crate::services::audit::AuditService;

/// 活动公告服务
pub struct ActivityAnnouncementService;

impl ActivityAnnouncementService {
    /// 按班级列出活动公告
    pub async fn list_by_class(
        pool: &SqlitePool,
        input: ListActivityAnnouncementsInput,
    ) -> Result<Vec<ActivityAnnouncement>, AppError> {
        Self::validate_class_exists(pool, &input.class_id).await?;

        if let Some(audience) = input.audience.as_deref() {
            Self::validate_audience(audience)?;

            let items = sqlx::query_as::<_, ActivityAnnouncement>(
                "SELECT id, class_id, title, topic, audience, draft, adopted_text, template_id, status, is_deleted, created_at, updated_at FROM activity_announcement WHERE class_id = ? AND audience = ? AND is_deleted = 0 ORDER BY created_at DESC",
            )
            .bind(&input.class_id)
            .bind(audience)
            .fetch_all(pool)
            .await?;

            return Ok(items);
        }

        let items = sqlx::query_as::<_, ActivityAnnouncement>(
            "SELECT id, class_id, title, topic, audience, draft, adopted_text, template_id, status, is_deleted, created_at, updated_at FROM activity_announcement WHERE class_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(&input.class_id)
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 根据ID查询活动公告
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<ActivityAnnouncement, AppError> {
        let item = sqlx::query_as::<_, ActivityAnnouncement>(
            "SELECT id, class_id, title, topic, audience, draft, adopted_text, template_id, status, is_deleted, created_at, updated_at FROM activity_announcement WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("活动公告不存在：{id}")))?;

        Ok(item)
    }

    /// 创建活动公告
    pub async fn create(
        pool: &SqlitePool,
        input: CreateActivityAnnouncementInput,
    ) -> Result<ActivityAnnouncement, AppError> {
        Self::validate_class_exists(pool, &input.class_id).await?;

        let status = input.status.unwrap_or_else(|| String::from("draft"));
        Self::validate_status(&status)?;

        let audience = input.audience.unwrap_or_else(|| String::from("parent"));
        Self::validate_audience(&audience)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO activity_announcement (id, class_id, title, topic, audience, draft, adopted_text, template_id, status, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.class_id)
        .bind(&input.title)
        .bind(&input.topic)
        .bind(&audience)
        .bind(&input.draft)
        .bind(&input.adopted_text)
        .bind(&input.template_id)
        .bind(&status)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_activity_announcement",
            "activity_announcement",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 更新活动公告
    pub async fn update(
        pool: &SqlitePool,
        input: UpdateActivityAnnouncementInput,
    ) -> Result<ActivityAnnouncement, AppError> {
        let has_updates = input.title.is_some()
            || input.topic.is_some()
            || input.audience.is_some()
            || input.draft.is_some()
            || input.adopted_text.is_some()
            || input.template_id.is_some()
            || input.status.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        if let Some(status) = input.status.as_deref() {
            Self::validate_status(status)?;
        }

        if let Some(audience) = input.audience.as_deref() {
            Self::validate_audience(audience)?;
        }

        Self::get_by_id(pool, &input.id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE activity_announcement SET title = COALESCE(?, title), topic = COALESCE(?, topic), audience = COALESCE(?, audience), draft = COALESCE(?, draft), adopted_text = COALESCE(?, adopted_text), template_id = COALESCE(?, template_id), status = COALESCE(?, status), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.title)
        .bind(&input.topic)
        .bind(&input.audience)
        .bind(&input.draft)
        .bind(&input.adopted_text)
        .bind(&input.template_id)
        .bind(&input.status)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("活动公告不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_activity_announcement",
            "activity_announcement",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    /// 软删除活动公告
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE activity_announcement SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("活动公告不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_activity_announcement",
            "activity_announcement",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 校验班级是否存在
    async fn validate_class_exists(pool: &SqlitePool, class_id: &str) -> Result<(), AppError> {
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

    /// 校验状态取值是否合法
    fn validate_status(status: &str) -> Result<(), AppError> {
        if status == "draft" || status == "adopted" || status == "rejected" {
            return Ok(());
        }

        Err(AppError::InvalidInput(format!(
            "status 非法：{status}，仅支持 draft/adopted/rejected"
        )))
    }

    /// 校验受众取值是否合法
    fn validate_audience(audience: &str) -> Result<(), AppError> {
        if audience == "parent" || audience == "student" || audience == "internal" {
            return Ok(());
        }

        Err(AppError::InvalidInput(format!(
            "audience 非法：{audience}，仅支持 parent/student/internal"
        )))
    }
}

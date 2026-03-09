//! 应用设置服务模块
//!
//! 提供应用设置的持久化 CRUD 能力并记录审计日志。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::app_settings::AppSetting;
use crate::services::audit::AuditService;

/// 应用设置服务。
pub struct AppSettingsService;

impl AppSettingsService {
    /// 根据 key 获取单个设置。
    pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<AppSetting, AppError> {
        let item = sqlx::query_as::<_, AppSetting>(
            "SELECT id, key, value, category, description, is_deleted, created_at, updated_at FROM app_settings WHERE key = ? AND is_deleted = 0",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("设置不存在：{key}")))?;

        Ok(item)
    }

    /// 根据分类获取设置列表。
    pub async fn get_settings_by_category(
        pool: &SqlitePool,
        category: &str,
    ) -> Result<Vec<AppSetting>, AppError> {
        let items = sqlx::query_as::<_, AppSetting>(
            "SELECT id, key, value, category, description, is_deleted, created_at, updated_at FROM app_settings WHERE category = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(category)
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 获取全部设置。
    pub async fn list_settings(pool: &SqlitePool) -> Result<Vec<AppSetting>, AppError> {
        let items = sqlx::query_as::<_, AppSetting>(
            "SELECT id, key, value, category, description, is_deleted, created_at, updated_at FROM app_settings WHERE is_deleted = 0 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 按 key 插入或更新设置。
    pub async fn upsert_setting(
        pool: &SqlitePool,
        key: &str,
        value: &str,
        category: &str,
        description: Option<&str>,
    ) -> Result<AppSetting, AppError> {
        if key.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("key 不能为空")));
        }
        if category.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("category 不能为空")));
        }

        let now = Utc::now().to_rfc3339();
        let existing = sqlx::query_as::<_, AppSetting>(
            "SELECT id, key, value, category, description, is_deleted, created_at, updated_at FROM app_settings WHERE key = ? AND is_deleted = 0",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?;

        let target_id = if let Some(item) = existing {
            sqlx::query(
                "UPDATE app_settings SET value = ?, category = ?, description = ?, updated_at = ? WHERE key = ? AND is_deleted = 0",
            )
            .bind(value)
            .bind(category)
            .bind(description)
            .bind(&now)
            .bind(key)
            .execute(pool)
            .await?;
            item.id
        } else {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO app_settings (id, key, value, category, description, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
            )
            .bind(&id)
            .bind(key)
            .bind(value)
            .bind(category)
            .bind(description)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
            id
        };

        AuditService::log(
            pool,
            "system",
            "upsert_app_setting",
            "app_settings",
            Some(&target_id),
            "medium",
            false,
        )
        .await?;

        Self::get_setting(pool, key).await
    }

    /// 软删除设置。
    pub async fn delete_setting(pool: &SqlitePool, key: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result =
            sqlx::query("UPDATE app_settings SET is_deleted = 1, updated_at = ? WHERE key = ? AND is_deleted = 0")
                .bind(&now)
                .bind(key)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("设置不存在：{key}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_app_setting",
            "app_settings",
            Some(key),
            "high",
            false,
        )
        .await?;

        Ok(())
    }
}

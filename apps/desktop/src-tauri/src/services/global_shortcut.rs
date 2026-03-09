//! 全局快捷键服务模块
//!
//! 提供全局快捷键的查询、创建、更新与软删除能力。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::global_shortcut::{
    CreateGlobalShortcutInput, GlobalShortcut, UpdateGlobalShortcutInput,
};
use crate::services::audit::AuditService;

/// 全局快捷键服务。
pub struct GlobalShortcutService;

impl GlobalShortcutService {
    /// 查询全部未删除的全局快捷键。
    pub async fn list_shortcuts(pool: &SqlitePool) -> Result<Vec<GlobalShortcut>, AppError> {
        let shortcuts = sqlx::query_as::<_, GlobalShortcut>(
            "SELECT id, action, key_combination, enabled, description, is_deleted, created_at, updated_at FROM global_shortcut WHERE is_deleted = 0 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(shortcuts)
    }

    /// 按 ID 查询单个全局快捷键。
    pub async fn get_shortcut(pool: &SqlitePool, id: &str) -> Result<GlobalShortcut, AppError> {
        let shortcut = sqlx::query_as::<_, GlobalShortcut>(
            "SELECT id, action, key_combination, enabled, description, is_deleted, created_at, updated_at FROM global_shortcut WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("全局快捷键不存在：{id}")))?;

        Ok(shortcut)
    }

    /// 按 action 查询单个全局快捷键。
    pub async fn get_shortcut_by_action(
        pool: &SqlitePool,
        action: &str,
    ) -> Result<GlobalShortcut, AppError> {
        let shortcut = sqlx::query_as::<_, GlobalShortcut>(
            "SELECT id, action, key_combination, enabled, description, is_deleted, created_at, updated_at FROM global_shortcut WHERE action = ? AND is_deleted = 0",
        )
        .bind(action)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("全局快捷键 action 不存在：{action}")))?;

        Ok(shortcut)
    }

    /// 创建全局快捷键。
    pub async fn create_shortcut(
        pool: &SqlitePool,
        input: CreateGlobalShortcutInput,
    ) -> Result<GlobalShortcut, AppError> {
        if input.action.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("action 不能为空")));
        }
        if input.key_combination.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from(
                "key_combination 不能为空",
            )));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let enabled = input.enabled.unwrap_or(1);

        sqlx::query(
            "INSERT INTO global_shortcut (id, action, key_combination, enabled, description, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.action)
        .bind(&input.key_combination)
        .bind(enabled)
        .bind(&input.description)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_global_shortcut",
            "global_shortcut",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_shortcut(pool, &id).await
    }

    /// 更新全局快捷键。
    pub async fn update_shortcut(
        pool: &SqlitePool,
        input: UpdateGlobalShortcutInput,
    ) -> Result<GlobalShortcut, AppError> {
        let has_updates = input.action.is_some()
            || input.key_combination.is_some()
            || input.enabled.is_some()
            || input.description.is_some();
        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }
        if let Some(action) = input.action.as_deref() {
            if action.trim().is_empty() {
                return Err(AppError::InvalidInput(String::from("action 不能为空")));
            }
        }
        if let Some(key_combination) = input.key_combination.as_deref() {
            if key_combination.trim().is_empty() {
                return Err(AppError::InvalidInput(String::from(
                    "key_combination 不能为空",
                )));
            }
        }

        Self::get_shortcut(pool, &input.id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE global_shortcut SET action = COALESCE(?, action), key_combination = COALESCE(?, key_combination), enabled = COALESCE(?, enabled), description = COALESCE(?, description), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.action)
        .bind(&input.key_combination)
        .bind(input.enabled)
        .bind(&input.description)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "全局快捷键不存在：{}",
                input.id
            )));
        }

        AuditService::log(
            pool,
            "system",
            "update_global_shortcut",
            "global_shortcut",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_shortcut(pool, &input.id).await
    }

    /// 软删除全局快捷键。
    pub async fn delete_shortcut(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE global_shortcut SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("全局快捷键不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_global_shortcut",
            "global_shortcut",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }
}

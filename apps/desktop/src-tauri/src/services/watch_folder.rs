//! 监控文件夹服务模块
//!
//! 提供监控文件夹的查询、创建、更新与软删除能力。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::watch_folder::{CreateWatchFolderInput, UpdateWatchFolderInput, WatchFolder};
use crate::services::audit::AuditService;

/// 监控文件夹服务。
pub struct WatchFolderService;

impl WatchFolderService {
    /// 查询全部未删除的监控文件夹。
    pub async fn list_folders(pool: &SqlitePool) -> Result<Vec<WatchFolder>, AppError> {
        let folders = sqlx::query_as::<_, WatchFolder>(
            "SELECT id, folder_path, pattern, action, enabled, is_deleted, created_at, updated_at FROM watch_folder WHERE is_deleted = 0 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(folders)
    }

    /// 按 ID 查询单个监控文件夹。
    pub async fn get_folder(pool: &SqlitePool, id: &str) -> Result<WatchFolder, AppError> {
        let folder = sqlx::query_as::<_, WatchFolder>(
            "SELECT id, folder_path, pattern, action, enabled, is_deleted, created_at, updated_at FROM watch_folder WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("监控文件夹不存在：{id}")))?;

        Ok(folder)
    }

    /// 创建监控文件夹。
    pub async fn create_folder(
        pool: &SqlitePool,
        input: CreateWatchFolderInput,
    ) -> Result<WatchFolder, AppError> {
        if input.folder_path.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("folder_path 不能为空")));
        }
        if input.action.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("action 不能为空")));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let enabled = input.enabled.unwrap_or(1);

        sqlx::query(
            "INSERT INTO watch_folder (id, folder_path, pattern, action, enabled, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.folder_path)
        .bind(&input.pattern)
        .bind(&input.action)
        .bind(enabled)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_watch_folder",
            "watch_folder",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_folder(pool, &id).await
    }

    /// 更新监控文件夹。
    pub async fn update_folder(
        pool: &SqlitePool,
        input: UpdateWatchFolderInput,
    ) -> Result<WatchFolder, AppError> {
        let has_updates = input.folder_path.is_some()
            || input.pattern.is_some()
            || input.action.is_some()
            || input.enabled.is_some();
        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }
        if let Some(folder_path) = input.folder_path.as_deref() {
            if folder_path.trim().is_empty() {
                return Err(AppError::InvalidInput(String::from("folder_path 不能为空")));
            }
        }
        if let Some(action) = input.action.as_deref() {
            if action.trim().is_empty() {
                return Err(AppError::InvalidInput(String::from("action 不能为空")));
            }
        }

        Self::get_folder(pool, &input.id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE watch_folder SET folder_path = COALESCE(?, folder_path), pattern = COALESCE(?, pattern), action = COALESCE(?, action), enabled = COALESCE(?, enabled), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.folder_path)
        .bind(&input.pattern)
        .bind(&input.action)
        .bind(input.enabled)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "监控文件夹不存在：{}",
                input.id
            )));
        }

        AuditService::log(
            pool,
            "system",
            "update_watch_folder",
            "watch_folder",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_folder(pool, &input.id).await
    }

    /// 软删除监控文件夹。
    pub async fn delete_folder(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE watch_folder SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("监控文件夹不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_watch_folder",
            "watch_folder",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }
}

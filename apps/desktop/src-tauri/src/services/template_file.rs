//! 校本模板文件服务模块
//!
//! 提供校本模板文件的列表查询、创建、更新、删除能力。

use chrono::Utc;
use sqlx::QueryBuilder;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::template_file::{
    CreateTemplateFileInput, ListTemplateFilesInput, TemplateFile, UpdateTemplateFileInput,
};
use crate::services::audit::AuditService;

/// 校本模板文件服务
pub struct TemplateFileService;

impl TemplateFileService {
    /// 列表查询模板文件，支持按类型和启用状态过滤
    pub async fn list(
        pool: &SqlitePool,
        input: ListTemplateFilesInput,
    ) -> Result<Vec<TemplateFile>, AppError> {
        let mut query = QueryBuilder::new(
            "SELECT id, type, school_scope, version, file_path, enabled, is_deleted, created_at FROM template_file WHERE is_deleted = 0",
        );

        if let Some(r#type) = input.r#type.as_deref() {
            query.push(" AND type = ").push_bind(r#type);
        }

        if let Some(enabled) = input.enabled {
            query.push(" AND enabled = ").push_bind(enabled);
        }

        query.push(" ORDER BY created_at DESC");

        let items = query
            .build_query_as::<TemplateFile>()
            .fetch_all(pool)
            .await?;

        Ok(items)
    }

    /// 按 ID 获取模板文件
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<TemplateFile, AppError> {
        let item = sqlx::query_as::<_, TemplateFile>(
            "SELECT id, type, school_scope, version, file_path, enabled, is_deleted, created_at FROM template_file WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("校本模板文件不存在：{id}")))?;

        Ok(item)
    }

    /// 创建模板文件
    pub async fn create(
        pool: &SqlitePool,
        input: CreateTemplateFileInput,
    ) -> Result<TemplateFile, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let enabled = input.enabled.unwrap_or(1);

        sqlx::query(
            "INSERT INTO template_file (id, type, school_scope, version, file_path, enabled, is_deleted, created_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(&id)
        .bind(&input.r#type)
        .bind(&input.school_scope)
        .bind(&input.version)
        .bind(&input.file_path)
        .bind(enabled)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_template_file",
            "template_file",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 更新模板文件
    pub async fn update(
        pool: &SqlitePool,
        input: UpdateTemplateFileInput,
    ) -> Result<TemplateFile, AppError> {
        let has_updates = input.r#type.is_some()
            || input.school_scope.is_some()
            || input.version.is_some()
            || input.file_path.is_some()
            || input.enabled.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        Self::get_by_id(pool, &input.id).await?;

        let result = sqlx::query(
            "UPDATE template_file SET type = COALESCE(?, type), school_scope = COALESCE(?, school_scope), version = COALESCE(?, version), file_path = COALESCE(?, file_path), enabled = COALESCE(?, enabled) WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.r#type)
        .bind(&input.school_scope)
        .bind(&input.version)
        .bind(&input.file_path)
        .bind(input.enabled)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "校本模板文件不存在：{}",
                input.id
            )));
        }

        AuditService::log(
            pool,
            "system",
            "update_template_file",
            "template_file",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    /// 软删除模板文件
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let result =
            sqlx::query("UPDATE template_file SET is_deleted = 1 WHERE id = ? AND is_deleted = 0")
                .bind(id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("校本模板文件不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_template_file",
            "template_file",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }
}

//! AI 参数预设服务模块
//!
//! 提供 AI 参数预设的持久化 CRUD、激活态切换与审计记录。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::ai_param_preset::{AiParamPreset, CreatePresetInput, UpdatePresetInput};
use crate::services::audit::AuditService;

/// AI 参数预设服务。
pub struct AiParamPresetService;

impl AiParamPresetService {
    /// 列出全部未删除预设。
    pub async fn list_presets(pool: &SqlitePool) -> Result<Vec<AiParamPreset>, AppError> {
        let items = sqlx::query_as::<_, AiParamPreset>(
            "SELECT id, name, display_name, temperature, top_p, max_tokens, is_default, is_active, is_deleted, created_at, updated_at FROM ai_param_preset WHERE is_deleted = 0 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 获取当前激活预设。
    pub async fn get_active_preset(pool: &SqlitePool) -> Result<AiParamPreset, AppError> {
        let item = sqlx::query_as::<_, AiParamPreset>(
            "SELECT id, name, display_name, temperature, top_p, max_tokens, is_default, is_active, is_deleted, created_at, updated_at FROM ai_param_preset WHERE is_active = 1 AND is_deleted = 0 ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(String::from("未找到激活的 AI 参数预设")))?;

        Ok(item)
    }

    /// 创建参数预设。
    pub async fn create_preset(
        pool: &SqlitePool,
        input: CreatePresetInput,
    ) -> Result<AiParamPreset, AppError> {
        Self::validate_name(&input.name)?;
        Self::validate_display_name(&input.display_name)?;
        Self::validate_temperature(input.temperature)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let is_default = i32::from(input.is_default.unwrap_or(false));
        let is_active = i32::from(input.is_active.unwrap_or(false));

        if is_active == 1 {
            sqlx::query(
                "UPDATE ai_param_preset SET is_active = 0, updated_at = ? WHERE is_deleted = 0",
            )
            .bind(&now)
            .execute(pool)
            .await?;
        }

        sqlx::query(
            "INSERT INTO ai_param_preset (id, name, display_name, temperature, top_p, max_tokens, is_default, is_active, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&input.display_name)
        .bind(input.temperature)
        .bind(input.top_p)
        .bind(input.max_tokens)
        .bind(is_default)
        .bind(is_active)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_ai_param_preset",
            "ai_param_preset",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 更新参数预设（支持激活态切换，保证仅一个激活）。
    pub async fn update_preset(
        pool: &SqlitePool,
        input: UpdatePresetInput,
    ) -> Result<AiParamPreset, AppError> {
        let has_updates = input.name.is_some()
            || input.display_name.is_some()
            || input.temperature.is_some()
            || input.top_p.is_some()
            || input.max_tokens.is_some()
            || input.is_default.is_some()
            || input.is_active.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        let existing = Self::get_by_id(pool, &input.id).await?;

        if let Some(name) = input.name.as_deref() {
            Self::validate_name(name)?;
        }
        if let Some(display_name) = input.display_name.as_deref() {
            Self::validate_display_name(display_name)?;
        }
        if let Some(temperature) = input.temperature {
            Self::validate_temperature(temperature)?;
        }

        if existing.is_default == 1 && input.is_default == Some(false) {
            return Err(AppError::InvalidInput(String::from(
                "系统默认预设不允许取消默认标记",
            )));
        }

        let now = Utc::now().to_rfc3339();
        let is_default = input.is_default.map(i32::from);
        let is_active = input.is_active.map(i32::from);

        if is_active == Some(1) {
            sqlx::query(
                "UPDATE ai_param_preset SET is_active = 0, updated_at = ? WHERE id <> ? AND is_deleted = 0",
            )
            .bind(&now)
            .bind(&input.id)
            .execute(pool)
            .await?;
        }

        let result = sqlx::query(
            "UPDATE ai_param_preset SET name = COALESCE(?, name), display_name = COALESCE(?, display_name), temperature = COALESCE(?, temperature), top_p = COALESCE(?, top_p), max_tokens = COALESCE(?, max_tokens), is_default = COALESCE(?, is_default), is_active = COALESCE(?, is_active), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.name)
        .bind(&input.display_name)
        .bind(input.temperature)
        .bind(input.top_p)
        .bind(input.max_tokens)
        .bind(is_default)
        .bind(is_active)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "AI 参数预设不存在：{}",
                input.id
            )));
        }

        AuditService::log(
            pool,
            "system",
            "update_ai_param_preset",
            "ai_param_preset",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    /// 软删除参数预设（禁止删除系统默认预设）。
    pub async fn delete_preset(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let existing = Self::get_by_id(pool, id).await?;
        if existing.is_default == 1 {
            return Err(AppError::PermissionDenied(String::from(
                "系统默认预设不允许删除",
            )));
        }

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE ai_param_preset SET is_deleted = 1, is_active = 0, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("AI 参数预设不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_ai_param_preset",
            "ai_param_preset",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 激活指定预设并取消其他预设激活态。
    pub async fn activate_preset(pool: &SqlitePool, id: &str) -> Result<AiParamPreset, AppError> {
        Self::get_by_id(pool, id).await?;

        let now = Utc::now().to_rfc3339();
        let mut tx = pool.begin().await?;

        sqlx::query(
            "UPDATE ai_param_preset SET is_active = 0, updated_at = ? WHERE is_deleted = 0",
        )
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        let result =
            sqlx::query("UPDATE ai_param_preset SET is_active = 1, updated_at = ? WHERE id = ? AND is_deleted = 0")
                .bind(&now)
                .bind(id)
                .execute(&mut *tx)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("AI 参数预设不存在：{id}")));
        }

        tx.commit().await?;

        AuditService::log(
            pool,
            "system",
            "activate_ai_param_preset",
            "ai_param_preset",
            Some(id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, id).await
    }

    /// 根据 ID 获取预设。
    async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<AiParamPreset, AppError> {
        let item = sqlx::query_as::<_, AiParamPreset>(
            "SELECT id, name, display_name, temperature, top_p, max_tokens, is_default, is_active, is_deleted, created_at, updated_at FROM ai_param_preset WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("AI 参数预设不存在：{id}")))?;

        Ok(item)
    }

    /// 校验预设名称。
    fn validate_name(name: &str) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("name 不能为空")));
        }
        Ok(())
    }

    /// 校验预设显示名称。
    fn validate_display_name(display_name: &str) -> Result<(), AppError> {
        if display_name.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from(
                "display_name 不能为空",
            )));
        }
        Ok(())
    }

    /// 校验温度范围。
    fn validate_temperature(temperature: f64) -> Result<(), AppError> {
        if !(0.0..=2.0).contains(&temperature) {
            return Err(AppError::InvalidInput(String::from(
                "temperature 必须在 0.0 到 2.0 之间",
            )));
        }
        Ok(())
    }
}

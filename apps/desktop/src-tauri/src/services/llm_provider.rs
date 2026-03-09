//! LLM Provider 适配层服务
//!
//! 通过 rig-core 提供统一的 LLM 调用能力，并实现 AI 配置持久化。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use rig::client::CompletionClient;
use rig::providers::openai;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::ai_config::{AiConfig, AiConfigSafe, CreateAiConfigInput, UpdateAiConfigInput};
use crate::services::audit::AuditService;
use crate::services::keychain::KeychainService;

/// 密钥迁移报告。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MigrationReport {
    pub scanned: i32,
    pub migrated: i32,
    pub failed: i32,
}

/// LLM Provider 服务。
pub struct LlmProviderService;

impl LlmProviderService {
    /// 获取当前激活的 AI 配置。
    pub async fn get_active_config(pool: &SqlitePool) -> Result<AiConfig, AppError> {
        let config = sqlx::query_as::<_, AiConfig>(
            "SELECT id, provider_name, display_name, base_url, api_key_encrypted, default_model, is_active, config_json, is_deleted, created_at, updated_at FROM ai_config WHERE is_active = 1 AND is_deleted = 0 ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(String::from("未找到已激活的 AI Provider 配置")))?;

        Ok(config)
    }

    /// 获取全部 AI 配置（安全版本）。
    pub async fn list_configs(pool: &SqlitePool) -> Result<Vec<AiConfigSafe>, AppError> {
        let configs = sqlx::query_as::<_, AiConfig>(
            "SELECT id, provider_name, display_name, base_url, api_key_encrypted, default_model, is_active, config_json, is_deleted, created_at, updated_at FROM ai_config WHERE is_deleted = 0 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(configs
            .iter()
            .map(|config| {
                let mut safe = Self::to_safe(config);
                safe.has_api_key = KeychainService::has_api_key(&config.id)
                    || !config.api_key_encrypted.trim().is_empty();
                safe
            })
            .collect())
    }

    /// 创建 AI 配置。
    pub async fn create_config(
        pool: &SqlitePool,
        input: CreateAiConfigInput,
    ) -> Result<AiConfigSafe, AppError> {
        Self::validate_provider_name(&input.provider_name)?;
        Self::validate_required(&input.display_name, "display_name")?;
        Self::validate_required(&input.base_url, "base_url")?;
        Self::validate_required(&input.api_key, "api_key")?;
        Self::validate_required(&input.default_model, "default_model")?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let is_active = i32::from(input.is_active.unwrap_or(false));
        let mut encoded_key = String::new();

        if KeychainService::store_api_key(&id, &input.api_key).is_err() {
            eprintln!("[LlmProviderService] Keychain 不可用，回退 Base64 存储");
            encoded_key = Self::encode_api_key(&input.api_key);
        }

        if is_active == 1 {
            sqlx::query("UPDATE ai_config SET is_active = 0, updated_at = ? WHERE is_deleted = 0")
                .bind(&now)
                .execute(pool)
                .await?;
        }

        sqlx::query(
            "INSERT INTO ai_config (id, provider_name, display_name, base_url, api_key_encrypted, default_model, is_active, config_json, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.provider_name)
        .bind(&input.display_name)
        .bind(&input.base_url)
        .bind(&encoded_key)
        .bind(&input.default_model)
        .bind(is_active)
        .bind(&input.config_json)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_ai_config",
            "ai_config",
            Some(&id),
            "high",
            false,
        )
        .await?;

        let created = Self::get_by_id(pool, &id).await?;
        Ok(Self::to_safe(&created))
    }

    /// 更新 AI 配置。
    pub async fn update_config(
        pool: &SqlitePool,
        input: UpdateAiConfigInput,
    ) -> Result<AiConfigSafe, AppError> {
        let has_updates = input.display_name.is_some()
            || input.base_url.is_some()
            || input.api_key.is_some()
            || input.default_model.is_some()
            || input.is_active.is_some()
            || input.config_json.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        Self::get_by_id(pool, &input.id).await?;

        if let Some(name) = input.display_name.as_deref() {
            Self::validate_required(name, "display_name")?;
        }
        if let Some(url) = input.base_url.as_deref() {
            Self::validate_required(url, "base_url")?;
        }
        if let Some(model) = input.default_model.as_deref() {
            Self::validate_required(model, "default_model")?;
        }
        if let Some(key) = input.api_key.as_deref() {
            Self::validate_required(key, "api_key")?;
        }

        let now = Utc::now().to_rfc3339();
        let is_active = input.is_active.map(i32::from);

        let mut encoded_key: Option<String> = None;
        if let Some(api_key) = input.api_key.as_deref() {
            if KeychainService::store_api_key(&input.id, api_key).is_err() {
                eprintln!("[LlmProviderService] Keychain 更新失败，回退 Base64 存储");
                encoded_key = Some(Self::encode_api_key(api_key));
            }
        }

        if is_active == Some(1) {
            sqlx::query(
                "UPDATE ai_config SET is_active = 0, updated_at = ? WHERE id <> ? AND is_deleted = 0",
            )
            .bind(&now)
            .bind(&input.id)
            .execute(pool)
            .await?;
        }

        let result = sqlx::query(
            "UPDATE ai_config SET display_name = COALESCE(?, display_name), base_url = COALESCE(?, base_url), api_key_encrypted = COALESCE(?, api_key_encrypted), default_model = COALESCE(?, default_model), is_active = COALESCE(?, is_active), config_json = COALESCE(?, config_json), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.display_name)
        .bind(&input.base_url)
        .bind(&encoded_key)
        .bind(&input.default_model)
        .bind(is_active)
        .bind(&input.config_json)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("AI 配置不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_ai_config",
            "ai_config",
            Some(&input.id),
            "high",
            false,
        )
        .await?;

        let updated = Self::get_by_id(pool, &input.id).await?;
        Ok(Self::to_safe(&updated))
    }

    /// 软删除 AI 配置。
    pub async fn delete_config(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let _ = KeychainService::delete_api_key(id);

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE ai_config SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("AI 配置不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_ai_config",
            "ai_config",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 创建 rig OpenAI Chat Completions API 兼容客户端。
    ///
    /// DeepSeek/Qwen 等国产模型使用 Chat Completions API（非 Responses API），
    /// 因此通过 `completions_api()` 切换到兼容模式。
    pub fn create_client(config: &AiConfig) -> Result<openai::CompletionsClient, AppError> {
        let api_key = if KeychainService::has_api_key(&config.id) {
            KeychainService::get_api_key(&config.id)?
        } else {
            Self::decode_api_key(&config.api_key_encrypted)?
        };

        let responses_client = openai::Client::builder()
            .api_key(&api_key)
            .base_url(&config.base_url)
            .build()
            .map_err(|e| AppError::ExternalService(format!("创建 LLM 客户端失败：{e}")))?;

        // 转换为 Chat Completions API 客户端（兼容 DeepSeek/Qwen 等）
        Ok(responses_client.completions_api())
    }

    /// 创建指定模型的 Agent（使用 Chat Completions API）。
    pub fn create_agent(
        client: &openai::CompletionsClient,
        model: &str,
        preamble: &str,
        temperature: f64,
    ) -> rig::agent::Agent<openai::completion::CompletionModel> {
        client
            .agent(model)
            .preamble(preamble)
            .temperature(temperature)
            .build()
    }

    /// 按 ID 获取配置记录。
    async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<AiConfig, AppError> {
        let item = sqlx::query_as::<_, AiConfig>(
            "SELECT id, provider_name, display_name, base_url, api_key_encrypted, default_model, is_active, config_json, is_deleted, created_at, updated_at FROM ai_config WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("AI 配置不存在：{id}")))?;

        Ok(item)
    }

    /// 迁移历史 Base64 API Key 到系统密钥链。
    pub async fn migrate_keys_to_keychain(pool: &SqlitePool) -> Result<MigrationReport, AppError> {
        let configs = sqlx::query_as::<_, AiConfig>(
            "SELECT id, provider_name, display_name, base_url, api_key_encrypted, default_model, is_active, config_json, is_deleted, created_at, updated_at FROM ai_config WHERE is_deleted = 0",
        )
        .fetch_all(pool)
        .await?;

        let mut report = MigrationReport {
            scanned: configs.len() as i32,
            migrated: 0,
            failed: 0,
        };

        for config in configs {
            if KeychainService::has_api_key(&config.id) {
                continue;
            }

            let decoded = match Self::decode_api_key(&config.api_key_encrypted) {
                Ok(value) => value,
                Err(_) => {
                    report.failed += 1;
                    continue;
                }
            };

            match KeychainService::store_api_key(&config.id, &decoded) {
                Ok(_) => {
                    let now = Utc::now().to_rfc3339();
                    let _ = sqlx::query(
                        "UPDATE ai_config SET api_key_encrypted = '', updated_at = ? WHERE id = ? AND is_deleted = 0",
                    )
                    .bind(&now)
                    .bind(&config.id)
                    .execute(pool)
                    .await;
                    report.migrated += 1;
                }
                Err(_) => {
                    report.failed += 1;
                }
            }
        }

        Ok(report)
    }

    /// 将明文 API Key 编码为 Base64。
    fn encode_api_key(raw: &str) -> String {
        BASE64.encode(raw.as_bytes())
    }

    /// 将 Base64 编码的 API Key 解码为明文。
    fn decode_api_key(encoded: &str) -> Result<String, AppError> {
        let bytes = BASE64.decode(encoded).map_err(|e| {
            AppError::Config(format!("解码 API Key 失败，配置已损坏或格式非法：{e}"))
        })?;

        String::from_utf8(bytes)
            .map_err(|e| AppError::Config(format!("API Key 非法（非 UTF-8 字符串）：{e}")))
    }

    /// 转换为安全输出结构，隐藏密钥正文。
    fn to_safe(config: &AiConfig) -> AiConfigSafe {
        AiConfigSafe {
            id: config.id.clone(),
            provider_name: config.provider_name.clone(),
            display_name: config.display_name.clone(),
            base_url: config.base_url.clone(),
            has_api_key: !config.api_key_encrypted.trim().is_empty(),
            default_model: config.default_model.clone(),
            is_active: config.is_active,
            config_json: config.config_json.clone(),
            created_at: config.created_at.clone(),
            updated_at: config.updated_at.clone(),
        }
    }

    /// 验证 provider_name 是否合法。
    fn validate_provider_name(name: &str) -> Result<(), AppError> {
        if name == "deepseek" || name == "qwen" || name == "openai" || name == "custom" {
            return Ok(());
        }

        Err(AppError::InvalidInput(format!(
            "provider_name 非法：{name}，仅支持 deepseek/qwen/openai/custom",
        )))
    }

    /// 验证必填字符串字段。
    fn validate_required(value: &str, field_name: &str) -> Result<(), AppError> {
        if value.trim().is_empty() {
            return Err(AppError::InvalidInput(format!("{field_name} 不能为空")));
        }

        Ok(())
    }
}

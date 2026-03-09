//! 脱敏服务模块。
//!
//! 提供配置开关读取与基础正则脱敏能力。

use regex::Regex;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::services::app_settings::AppSettingsService;

/// 脱敏服务。
pub struct DesensitizeService;

impl DesensitizeService {
    /// 检查脱敏功能是否启用（从 app_settings 读取）。
    pub async fn is_enabled(pool: &SqlitePool) -> Result<bool, AppError> {
        let setting = AppSettingsService::get_setting(pool, "desensitize_enabled").await?;
        Ok(parse_bool_setting(&setting.value))
    }

    /// 对文本内容进行脱敏处理。
    pub fn desensitize_text(text: &str) -> String {
        let mut output = text.to_string();

        if let Ok(phone_re) = Regex::new(r"\b(1\d{2})\d{4}(\d{4})\b") {
            output = phone_re.replace_all(&output, "$1****$2").to_string();
        }

        if let Ok(id_re) = Regex::new(r"\b(\d{4})\d{10}(\w{4})\b") {
            output = id_re.replace_all(&output, "$1****$2").to_string();
        }

        if let Ok(email_re) =
            Regex::new(r"\b([A-Za-z0-9])[A-Za-z0-9._%+-]*(@[A-Za-z0-9.-]+\.[A-Za-z]{2,})\b")
        {
            output = email_re.replace_all(&output, "$1***$2").to_string();
        }

        if let Ok(name_re) = Regex::new(r"\b([\u4e00-\u9fa5])([\u4e00-\u9fa5]{1,3})\b") {
            output = name_re
                .replace_all(&output, |caps: &regex::Captures<'_>| {
                    let first = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                    let rest_len = caps.get(2).map(|m| m.as_str().chars().count()).unwrap_or(0);
                    format!("{}{}", first, "*".repeat(rest_len))
                })
                .to_string();
        }

        output
    }

    /// 如果脱敏开关启用，对文本进行脱敏。
    pub async fn desensitize_if_enabled(pool: &SqlitePool, text: &str) -> Result<String, AppError> {
        if Self::is_enabled(pool).await? {
            Ok(Self::desensitize_text(text))
        } else {
            Ok(text.to_string())
        }
    }
}

/// 解析布尔设置值，兼容 JSON 字符串与纯文本。
fn parse_bool_setting(value: &str) -> bool {
    if let Ok(parsed) = serde_json::from_str::<bool>(value) {
        return parsed;
    }

    match value.trim().trim_matches('"').to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => true,
        "false" | "0" | "no" | "off" => false,
        _ => false,
    }
}

//! 高危操作确认闸门服务模块。
//!
//! 提供高危动作识别与确认开关判定能力。

use sqlx::SqlitePool;

use crate::error::AppError;
use crate::services::app_settings::AppSettingsService;

/// 高危操作确认闸门服务。
pub struct HighRiskGateService;

impl HighRiskGateService {
    /// 检查高危确认是否启用（从 app_settings 读取）。
    pub async fn is_enabled(pool: &SqlitePool) -> Result<bool, AppError> {
        let setting = AppSettingsService::get_setting(pool, "high_risk_confirm_enabled").await?;
        Ok(parse_bool_setting(&setting.value))
    }

    /// 检查操作是否需要用户确认。
    pub async fn requires_confirmation(pool: &SqlitePool, action: &str) -> Result<bool, AppError> {
        if !Self::is_enabled(pool).await? {
            return Ok(false);
        }

        Ok(Self::high_risk_actions().contains(&action))
    }

    /// 定义高危操作列表。
    fn high_risk_actions() -> Vec<&'static str> {
        vec![
            "erase_workspace",
            "export_workspace",
            "batch_modify",
            "send_external",
            "delete_student",
        ]
    }
}

/// 解析布尔设置值，兼容 JSON 布尔或字符串。
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

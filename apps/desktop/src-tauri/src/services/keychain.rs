//! Keychain 密钥管理服务
//!
//! 优先使用操作系统密钥链保存 API Key，不可用时回退数据库 Base64 存储。

use keyring::Entry;

use crate::error::AppError;

const SERVICE_NAME: &str = "pureworker";

/// Keychain 服务。
pub struct KeychainService;

impl KeychainService {
    /// 存储 API Key 到系统密钥链。
    pub fn store_api_key(provider_id: &str, api_key: &str) -> Result<(), AppError> {
        let entry = Entry::new(SERVICE_NAME, provider_id)
            .map_err(|error| AppError::Config(format!("初始化密钥链失败：{error}")))?;
        entry
            .set_password(api_key)
            .map_err(|error| AppError::ExternalService(format!("写入密钥链失败：{error}")))
    }

    /// 从系统密钥链获取 API Key。
    pub fn get_api_key(provider_id: &str) -> Result<String, AppError> {
        let entry = Entry::new(SERVICE_NAME, provider_id)
            .map_err(|error| AppError::Config(format!("初始化密钥链失败：{error}")))?;
        entry
            .get_password()
            .map_err(|error| AppError::NotFound(format!("密钥链中未找到 API Key：{error}")))
    }

    /// 删除系统密钥链中的 API Key。
    pub fn delete_api_key(provider_id: &str) -> Result<(), AppError> {
        let entry = Entry::new(SERVICE_NAME, provider_id)
            .map_err(|error| AppError::Config(format!("初始化密钥链失败：{error}")))?;
        entry
            .delete_credential()
            .map_err(|error| AppError::ExternalService(format!("删除密钥链凭据失败：{error}")))
    }

    /// 检查密钥链中是否存在 API Key。
    pub fn has_api_key(provider_id: &str) -> bool {
        Self::get_api_key(provider_id).is_ok()
    }

    /// 判断系统密钥链是否可用。
    pub fn is_available() -> bool {
        let probe = "__pureworker_keychain_probe__";
        let entry = match Entry::new(SERVICE_NAME, probe) {
            Ok(entry) => entry,
            Err(_) => return false,
        };

        if entry.set_password("probe").is_err() {
            return false;
        }

        let available = entry.get_password().is_ok();
        let _ = entry.delete_credential();
        available
    }
}

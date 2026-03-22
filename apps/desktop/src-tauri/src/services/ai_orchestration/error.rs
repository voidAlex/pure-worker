//! AI 编排层错误定义
//!
//! 将运行时内部错误统一收口，并映射为上层 `AppError`。

use thiserror::Error;

use crate::error::AppError;

/// 编排层统一 Result 别名
pub type OrchestrationResult<T> = Result<T, OrchestrationError>;

/// 编排层错误枚举
#[derive(Debug, Error, Clone)]
pub enum OrchestrationError {
    #[error("请求参数无效：{0}")]
    InvalidRequest(String),

    #[error("未找到运行时 Profile：{0}")]
    ProfileNotFound(String),

    #[error("模型能力不足：{0}")]
    ModelCapabilityInsufficient(String),

    #[error("Provider 不可用：{0}")]
    ProviderUnavailable(String),

    #[error("工具暴露策略错误：{0}")]
    ToolExposure(String),

    #[error("执行存储错误：{0}")]
    Store(String),

    #[error("会话事件错误：{0}")]
    EventBus(String),

    #[error("编排内部错误：{0}")]
    Internal(String),
}

impl OrchestrationError {
    /// 转换到 AppError，供 IPC 命令层统一返回
    pub fn to_app_error(&self) -> AppError {
        match self {
            Self::InvalidRequest(message) => AppError::InvalidInput(message.clone()),
            Self::ProfileNotFound(message) => AppError::NotFound(message.clone()),
            Self::ModelCapabilityInsufficient(message) => AppError::InvalidInput(message.clone()),
            Self::ProviderUnavailable(message) => AppError::ExternalService(message.clone()),
            Self::ToolExposure(message) => AppError::TaskExecution(message.clone()),
            Self::Store(message) => AppError::Database(message.clone()),
            Self::EventBus(message) => AppError::TaskExecution(message.clone()),
            Self::Internal(message) => AppError::Internal(message.clone()),
        }
    }
}

impl From<OrchestrationError> for AppError {
    fn from(error: OrchestrationError) -> Self {
        error.to_app_error()
    }
}

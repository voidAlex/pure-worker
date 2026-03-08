//! 应用错误定义模块
//!
//! 定义所有业务错误类型，包含数据库、配置、文件操作、任务执行等错误

use serde::Serialize;
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error, Type, Clone, Serialize)]
pub enum AppError {
    #[error("数据库错误：{0}")]
    Database(String),

    #[error("配置错误：{0}")]
    Config(String),

    #[error("文件操作错误：{0}")]
    FileOperation(String),

    #[error("任务执行错误：{0}")]
    TaskExecution(String),

    #[error("参数无效：{0}")]
    InvalidInput(String),

    #[error("资源未找到：{0}")]
    NotFound(String),

    #[error("权限不足：{0}")]
    PermissionDenied(String),

    #[error("外部服务错误：{0}")]
    ExternalService(String),

    #[error("内部错误：{0}")]
    Internal(String),
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error.to_string())
    }
}

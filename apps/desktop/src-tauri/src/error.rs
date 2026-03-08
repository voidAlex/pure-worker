use serde::Serialize;
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error, Type)]
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

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("AppError", 2)?;
        let (code, message) = match self {
            Self::Database(msg) => ("DATABASE_ERROR", format!("数据库错误：{msg}")),
            Self::Config(msg) => ("CONFIG_ERROR", format!("配置错误：{msg}")),
            Self::FileOperation(msg) => ("FILE_ERROR", format!("文件操作错误：{msg}")),
            Self::TaskExecution(msg) => ("TASK_ERROR", format!("任务执行错误：{msg}")),
            Self::InvalidInput(msg) => ("INVALID_INPUT", format!("参数无效：{msg}")),
            Self::NotFound(msg) => ("NOT_FOUND", format!("资源未找到：{msg}")),
            Self::PermissionDenied(msg) => ("PERMISSION_DENIED", format!("权限不足：{msg}")),
            Self::ExternalService(msg) => ("EXTERNAL_ERROR", format!("外部服务错误：{msg}")),
            Self::Internal(msg) => ("INTERNAL_ERROR", format!("内部错误：{msg}")),
        };

        state.serialize_field("code", code)?;
        state.serialize_field("message", &message)?;
        state.end()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error.to_string())
    }
}

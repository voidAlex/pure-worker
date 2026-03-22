//! 执行运行时数据模型
//!
//! 定义 AI 执行运行时的核心数据结构，包括执行请求、会话、消息、记录和事件

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::FromRow;

/// 执行入口点枚举
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ExecutionEntrypoint {
    Chat,
    Grading,
    Communication,
    Search,
}

/// 流式模式枚举
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum StreamMode {
    Streaming,
    NonStreaming,
}

/// 执行状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Completed,
    Failed,
    Cancelled,
}

/// 会话事件版本常量
pub const SESSION_EVENT_VERSION: u32 = 1;

/// 会话事件枚举
/// 统一的事件协议，用于流式和非流式执行
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum SessionEvent {
    /// 执行开始
    Start { version: u32, message_id: String },
    /// 思考状态更新
    ThinkingStatus {
        version: u32,
        stage: String,
        description: String,
    },
    /// 工具调用
    ToolCall {
        version: u32,
        tool_name: String,
        input: serde_json::Value,
    },
    /// 工具调用结果
    ToolResult {
        version: u32,
        tool_name: String,
        output: String,
        success: bool,
    },
    /// 搜索摘要
    SearchSummary {
        version: u32,
        sources: Vec<String>,
        evidence_count: usize,
    },
    /// 推理摘要
    Reasoning { version: u32, summary: String },
    /// 内容片段（流式输出）
    Chunk { version: u32, content: String },
    /// 执行摘要
    ExecutionSummary {
        version: u32,
        status: String,
        used_model: String,
    },
    /// 执行完成
    Complete { version: u32 },
    /// 执行错误
    Error { version: u32, message: String },
}

/// 执行附件
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExecutionAttachment {
    /// 文件路径
    pub path: String,
    /// 媒体类型（MIME type）
    pub media_type: Option<String>,
    /// 显示名称
    pub display_name: Option<String>,
}

/// 执行请求输入
#[derive(Debug, Deserialize, Type)]
pub struct ExecutionRequest {
    /// 会话ID（流式续聊时必填；新会话可为空）
    pub session_id: Option<String>,
    /// 执行入口点
    pub entrypoint: ExecutionEntrypoint,
    /// Agent Profile ID
    pub agent_profile_id: String,
    /// 用户输入内容
    pub user_input: String,
    /// 附件列表
    pub attachments: Vec<ExecutionAttachment>,
    /// 是否启用 Agentic Search 自动检索
    pub use_agentic_search: bool,
    /// 流式模式
    pub stream_mode: StreamMode,
    /// 元数据JSON（可选，仅允许对象类型）
    pub metadata_json: Option<serde_json::Value>,
}

/// 执行响应输出
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ExecutionResponse {
    /// AI 回复内容
    pub content: String,
    /// 使用的模型名称
    pub model: String,
    /// 执行状态
    pub status: ExecutionStatus,
}

/// 执行会话实体
/// 对应数据库表 execution_session
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct ExecutionSession {
    /// 会话ID（UUID）
    pub id: String,
    /// 教师ID
    pub teacher_id: String,
    /// 会话标题
    pub title: Option<String>,
    /// 执行入口点
    pub entrypoint: String,
    /// Agent Profile ID
    pub agent_profile_id: String,
    /// 是否已删除（软删除）
    pub is_deleted: i32,
    /// 创建时间（ISO 8601）
    pub created_at: String,
    /// 更新时间（ISO 8601）
    pub updated_at: String,
}

/// 执行消息实体
/// 对应数据库表 execution_message
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct ExecutionMessage {
    /// 消息ID（UUID）
    pub id: String,
    /// 所属会话ID
    pub session_id: String,
    /// 角色（user/assistant）
    pub role: String,
    /// 内容
    pub content: String,
    /// 工具名称（如果使用工具）
    pub tool_name: Option<String>,
    /// 是否已删除（软删除）
    pub is_deleted: i32,
    /// 创建时间（ISO 8601）
    pub created_at: String,
}

/// 执行记录实体
/// 对应数据库表 execution_record
/// 存储每次 AI 执行的详细元数据和结果摘要
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct ExecutionRecord {
    /// 记录ID（UUID）
    pub id: String,
    /// 所属会话ID
    pub session_id: String,
    /// 关联的执行消息ID（assistant 消息）
    pub execution_message_id: String,
    /// 执行入口点
    pub entrypoint: String,
    /// Agent Profile ID
    pub agent_profile_id: String,
    /// 使用的模型ID
    pub model_id: String,
    /// 执行状态
    pub status: String,
    /// 推理摘要
    pub reasoning_summary: Option<String>,
    /// 搜索摘要JSON
    pub search_summary_json: Option<String>,
    /// 工具调用摘要JSON
    pub tool_calls_summary_json: Option<String>,
    /// 错误消息
    pub error_message: Option<String>,
    /// 元数据JSON（轻量快照）
    pub metadata_json: Option<String>,
    /// 创建时间（ISO 8601）
    pub created_at: String,
    /// 更新时间（ISO 8601）
    pub updated_at: String,
}

/// 创建执行会话输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateExecutionSessionInput {
    /// 教师ID
    pub teacher_id: String,
    /// 会话标题
    pub title: Option<String>,
    /// 执行入口点
    pub entrypoint: ExecutionEntrypoint,
    /// Agent Profile ID
    pub agent_profile_id: String,
}

/// 创建执行消息输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateExecutionMessageInput {
    /// 所属会话ID
    pub session_id: String,
    /// 角色
    pub role: String,
    /// 内容
    pub content: String,
    /// 工具名称
    pub tool_name: Option<String>,
}

/// 创建执行记录输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateExecutionRecordInput {
    /// 所属会话ID
    pub session_id: String,
    /// 关联的执行消息ID
    pub execution_message_id: String,
    /// 执行入口点
    pub entrypoint: ExecutionEntrypoint,
    /// Agent Profile ID
    pub agent_profile_id: String,
    /// 使用的模型ID
    pub model_id: String,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 推理摘要
    pub reasoning_summary: Option<String>,
    /// 搜索摘要JSON
    pub search_summary_json: Option<String>,
    /// 工具调用摘要JSON
    pub tool_calls_summary_json: Option<String>,
    /// 元数据JSON
    pub metadata_json: Option<serde_json::Value>,
}

/// 更新执行记录输入
#[derive(Debug, Deserialize, Type)]
pub struct UpdateExecutionRecordInput {
    /// 记录ID
    pub id: String,
    /// 执行状态
    pub status: Option<ExecutionStatus>,
    /// 推理摘要
    pub reasoning_summary: Option<String>,
    /// 搜索摘要JSON
    pub search_summary_json: Option<String>,
    /// 工具调用摘要JSON
    pub tool_calls_summary_json: Option<String>,
    /// 错误消息
    pub error_message: Option<String>,
}

/// 执行会话列表项
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct ExecutionSessionListItem {
    /// 会话ID
    pub id: String,
    /// 会话标题
    pub title: Option<String>,
    /// 执行入口点
    pub entrypoint: String,
    /// Agent Profile ID
    pub agent_profile_id: String,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 消息数量
    pub message_count: i64,
}

/// 执行消息列表项
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct ExecutionMessageListItem {
    /// 消息ID
    pub id: String,
    /// 角色
    pub role: String,
    /// 内容
    pub content: String,
    /// 工具名称
    pub tool_name: Option<String>,
    /// 创建时间
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 ExecutionEntrypoint 序列化
    #[test]
    fn test_execution_entrypoint_serialization() {
        let entrypoint = ExecutionEntrypoint::Chat;
        let json = serde_json::to_string(&entrypoint).unwrap();
        assert_eq!(json, "\"chat\"");
    }

    /// 测试 SessionEvent 构造
    #[test]
    fn test_session_event_construction() {
        let event = SessionEvent::Start {
            version: SESSION_EVENT_VERSION,
            message_id: "test-id".to_string(),
        };

        match event {
            SessionEvent::Start {
                version,
                message_id,
            } => {
                assert_eq!(version, 1);
                assert_eq!(message_id, "test-id");
            }
            _ => panic!("Expected Start event"),
        }
    }

    /// 测试 ExecutionRequest 字段约束
    #[test]
    fn test_execution_request_validation() {
        let request = ExecutionRequest {
            session_id: None,
            entrypoint: ExecutionEntrypoint::Chat,
            agent_profile_id: "chat.homeroom".to_string(),
            user_input: "测试输入".to_string(),
            attachments: vec![],
            use_agentic_search: false,
            stream_mode: StreamMode::Streaming,
            metadata_json: None,
        };

        assert!(request.session_id.is_none());
        assert_eq!(request.agent_profile_id, "chat.homeroom");
        assert!(!request.user_input.is_empty());
    }
}

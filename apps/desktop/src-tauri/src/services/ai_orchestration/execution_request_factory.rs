//! 执行请求工厂
//!
//! 负责从各种输入源（聊天输入、技能调用、外部请求）构建标准化的 ExecutionRequest。

use crate::models::conversation::ChatStreamInput;
use crate::models::execution::{
    ExecutionAttachment, ExecutionEntrypoint, ExecutionRequest, StreamMode,
};

/// 执行请求构建器
pub struct ExecutionRequestBuilder {
    session_id: Option<String>,
    entrypoint: ExecutionEntrypoint,
    agent_profile_id: String,
    user_input: String,
    attachments: Vec<ExecutionAttachment>,
    use_agentic_search: bool,
    stream_mode: StreamMode,
    metadata_json: Option<serde_json::Value>,
}

impl ExecutionRequestBuilder {
    /// 创建新的构建器，使用默认 Chat 入口点
    pub fn new(agent_profile_id: impl Into<String>, user_input: impl Into<String>) -> Self {
        Self {
            session_id: None,
            entrypoint: ExecutionEntrypoint::Chat,
            agent_profile_id: agent_profile_id.into(),
            user_input: user_input.into(),
            attachments: Vec::new(),
            use_agentic_search: false,
            stream_mode: StreamMode::Streaming,
            metadata_json: None,
        }
    }

    /// 设置会话ID（用于流式续聊）
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置入口点
    pub fn with_entrypoint(mut self, entrypoint: ExecutionEntrypoint) -> Self {
        self.entrypoint = entrypoint;
        self
    }

    /// 添加附件
    pub fn with_attachment(mut self, attachment: ExecutionAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// 添加多个附件
    pub fn with_attachments(mut self, attachments: Vec<ExecutionAttachment>) -> Self {
        self.attachments = attachments;
        self
    }

    /// 设置是否启用 Agentic Search
    pub fn with_agentic_search(mut self, enabled: bool) -> Self {
        self.use_agentic_search = enabled;
        self
    }

    /// 设置流式模式
    pub fn with_stream_mode(mut self, mode: StreamMode) -> Self {
        self.stream_mode = mode;
        self
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata_json = Some(metadata);
        self
    }

    /// 构建 ExecutionRequest
    pub fn build(self) -> ExecutionRequest {
        ExecutionRequest {
            session_id: self.session_id,
            entrypoint: self.entrypoint,
            agent_profile_id: self.agent_profile_id,
            user_input: self.user_input,
            attachments: self.attachments,
            use_agentic_search: self.use_agentic_search,
            stream_mode: self.stream_mode,
            metadata_json: self.metadata_json,
        }
    }
}

/// 执行请求工厂
pub struct ExecutionRequestFactory;

impl ExecutionRequestFactory {
    /// 从 ChatStreamInput 创建 ExecutionRequest
    ///
    /// 将旧的聊天输入格式转换为新的运行时请求格式
    pub fn from_chat_stream_input(
        input: &ChatStreamInput,
        session_id: Option<String>,
    ) -> ExecutionRequest {
        // 根据 agent_role 映射到对应的 agent_profile_id
        let agent_profile_id = map_role_to_profile(&input.agent_role);

        // 根据角色确定入口点
        let entrypoint = map_role_to_entrypoint(&input.agent_role);

        ExecutionRequestBuilder::new(agent_profile_id, input.message.clone())
            .with_session_id(session_id.unwrap_or_default())
            .with_entrypoint(entrypoint)
            .with_agentic_search(input.use_agentic_search.unwrap_or(false))
            .with_stream_mode(StreamMode::Streaming)
            .with_metadata(build_chat_metadata(input))
            .build()
    }

    /// 从基本参数创建 ExecutionRequest
    pub fn from_basic(
        agent_profile_id: impl Into<String>,
        user_input: impl Into<String>,
        session_id: Option<String>,
    ) -> ExecutionRequest {
        ExecutionRequestBuilder::new(agent_profile_id, user_input)
            .with_session_id(session_id.unwrap_or_default())
            .build()
    }

    /// 创建批改场景请求
    pub fn for_grading(
        user_input: impl Into<String>,
        attachments: Vec<ExecutionAttachment>,
        session_id: Option<String>,
    ) -> ExecutionRequest {
        ExecutionRequestBuilder::new("chat.grading", user_input)
            .with_session_id(session_id.unwrap_or_default())
            .with_entrypoint(ExecutionEntrypoint::Grading)
            .with_attachments(attachments)
            .build()
    }

    /// 创建家校沟通场景请求
    pub fn for_communication(
        user_input: impl Into<String>,
        session_id: Option<String>,
    ) -> ExecutionRequest {
        ExecutionRequestBuilder::new("chat.communication", user_input)
            .with_session_id(session_id.unwrap_or_default())
            .with_entrypoint(ExecutionEntrypoint::Communication)
            .build()
    }

    /// 创建搜索增强场景请求
    pub fn for_search(
        user_input: impl Into<String>,
        session_id: Option<String>,
    ) -> ExecutionRequest {
        ExecutionRequestBuilder::new("search.agentic", user_input)
            .with_session_id(session_id.unwrap_or_default())
            .with_entrypoint(ExecutionEntrypoint::Search)
            .with_agentic_search(true)
            .build()
    }
}

/// 将旧的角色标识映射到 Agent Profile ID
fn map_role_to_profile(agent_role: &str) -> String {
    match agent_role {
        "homeroom" => "chat.homeroom".to_string(),
        "grading" => "chat.grading".to_string(),
        "communication" => "chat.communication".to_string(),
        "ops" => "chat.ops".to_string(),
        _ => "chat.homeroom".to_string(),
    }
}

/// 将角色映射到入口点
fn map_role_to_entrypoint(agent_role: &str) -> ExecutionEntrypoint {
    match agent_role {
        "grading" => ExecutionEntrypoint::Grading,
        "communication" => ExecutionEntrypoint::Communication,
        "ops" => ExecutionEntrypoint::Chat,
        "homeroom" => ExecutionEntrypoint::Chat,
        _ => ExecutionEntrypoint::Chat,
    }
}

/// 构建聊天元数据
fn build_chat_metadata(input: &ChatStreamInput) -> serde_json::Value {
    serde_json::json!({
        "original_agent_role": input.agent_role,
        "use_agentic_search": input.use_agentic_search.unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证从 ChatStreamInput 正确转换请求
    #[test]
    fn test_from_chat_stream_input() {
        let chat_input = ChatStreamInput {
            conversation_id: Some("conv-123".to_string()),
            message: "测试消息".to_string(),
            agent_role: "grading".to_string(),
            use_agentic_search: Some(true),
        };

        let request = ExecutionRequestFactory::from_chat_stream_input(
            &chat_input,
            Some("session-456".to_string()),
        );

        assert_eq!(request.agent_profile_id, "chat.grading");
        assert_eq!(request.user_input, "测试消息");
        assert_eq!(request.session_id, Some("session-456".to_string()));
        assert!(request.use_agentic_search);
        assert!(matches!(request.entrypoint, ExecutionEntrypoint::Grading));
    }

    /// 验证角色到 Profile 的映射
    #[test]
    fn test_role_to_profile_mapping() {
        assert_eq!(map_role_to_profile("homeroom"), "chat.homeroom");
        assert_eq!(map_role_to_profile("grading"), "chat.grading");
        assert_eq!(map_role_to_profile("communication"), "chat.communication");
        assert_eq!(map_role_to_profile("ops"), "chat.ops");
        assert_eq!(map_role_to_profile("unknown"), "chat.homeroom");
    }

    /// 验证角色到入口点的映射
    #[test]
    fn test_role_to_entrypoint_mapping() {
        assert!(matches!(
            map_role_to_entrypoint("grading"),
            ExecutionEntrypoint::Grading
        ));
        assert!(matches!(
            map_role_to_entrypoint("communication"),
            ExecutionEntrypoint::Communication
        ));
        assert!(matches!(
            map_role_to_entrypoint("homeroom"),
            ExecutionEntrypoint::Chat
        ));
    }

    /// 验证构建器模式
    #[test]
    fn test_request_builder() {
        let request = ExecutionRequestBuilder::new("chat.homeroom", "你好")
            .with_session_id("session-1")
            .with_agentic_search(true)
            .with_stream_mode(StreamMode::NonStreaming)
            .with_metadata(serde_json::json!({"key": "value"}))
            .build();

        assert_eq!(request.agent_profile_id, "chat.homeroom");
        assert_eq!(request.user_input, "你好");
        assert_eq!(request.session_id, Some("session-1".to_string()));
        assert!(request.use_agentic_search);
        assert!(matches!(request.stream_mode, StreamMode::NonStreaming));
        assert!(request.metadata_json.is_some());
    }

    /// 验证批改场景工厂方法
    #[test]
    fn test_for_grading_factory() {
        let attachments = vec![ExecutionAttachment {
            path: "/tmp/test.jpg".to_string(),
            media_type: Some("image/jpeg".to_string()),
            display_name: Some("作业图片".to_string()),
        }];

        let request = ExecutionRequestFactory::for_grading("请批改", attachments.clone(), None);

        assert_eq!(request.agent_profile_id, "chat.grading");
        assert_eq!(request.attachments.len(), 1);
        assert!(matches!(request.entrypoint, ExecutionEntrypoint::Grading));
    }

    /// 验证搜索场景工厂方法
    #[test]
    fn test_for_search_factory() {
        let request =
            ExecutionRequestFactory::for_search("搜索内容", Some("session-2".to_string()));

        assert_eq!(request.agent_profile_id, "search.agentic");
        assert!(request.use_agentic_search);
        assert!(matches!(request.entrypoint, ExecutionEntrypoint::Search));
    }
}

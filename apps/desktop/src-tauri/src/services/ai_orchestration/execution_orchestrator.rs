//! 执行编排器
//!
//! 统一执行主链的核心编排服务，协调 profile 加载、模型路由、提示词组装、工具暴露和流式输出。

use std::collections::HashMap;
use std::pin::Pin;

use futures::stream::{Stream, StreamExt};
use rig::completion::ToolDefinition;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::execution::{
    CreateExecutionMessageInput, CreateExecutionRecordInput, CreateExecutionSessionInput,
    ExecutionEntrypoint, ExecutionStatus,
};
use crate::models::execution::{ExecutionRequest, SessionEvent, SESSION_EVENT_VERSION};
use crate::models::mcp_server::McpServerRecord;
use crate::services::agentic_search::AgenticSearchOrchestrator;
use crate::services::ai_orchestration::execution_store::ExecutionStoreService;
use crate::services::ai_orchestration::model_routing::{ModelRoutingService, RoutingCapability};
use crate::services::ai_orchestration::prompt_assembler::PromptAssemblerService;
use crate::services::ai_orchestration::session_event_bus::SessionEventBus;
use crate::services::ai_orchestration::tool_exposure::ToolExposureService;
use crate::services::ai_orchestration::AgentProfileResolver;
use crate::services::llm_provider::LlmProviderService;
use crate::services::provider_adapter::{AdapterFactory, ChatMessage, ChatRole};
use crate::services::tool_registry::ToolRegistry;

/// 运行时模型选择类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeModelSelection {
    /// 纯文本模型
    Text,
    /// 多模态/视觉模型
    Vision,
    /// 工具调用模型
    Tool,
    /// 推理模型
    Reasoning,
}

impl RuntimeModelSelection {
    /// 转换为路由能力
    fn to_routing_capability(self) -> RoutingCapability {
        match self {
            Self::Text => RoutingCapability::Text,
            Self::Vision => RoutingCapability::Vision,
            Self::Tool => RoutingCapability::Tool,
            Self::Reasoning => RoutingCapability::Reasoning,
        }
    }
}

/// 结构化执行产物
#[derive(Debug, Clone, Default)]
pub struct ExecutionArtifacts {
    /// 生成的内容
    pub content: String,
    /// 使用的模型ID
    pub model_id: String,
    /// 推理摘要
    pub reasoning_summary: Option<String>,
    /// 搜索摘要JSON
    pub search_summary_json: Option<String>,
    /// 工具调用摘要JSON
    pub tool_calls_summary_json: Option<String>,
}

/// 执行编排器
pub struct ExecutionOrchestrator<'a> {
    pool: &'a SqlitePool,
    profile_registry: &'a dyn AgentProfileResolver,
    event_bus: &'a SessionEventBus,
    tool_registry: &'a ToolRegistry,
    mcp_servers: HashMap<String, McpServerRecord>,
}

impl<'a> ExecutionOrchestrator<'a> {
    /// 创建新的执行编排器
    pub fn new(
        pool: &'a SqlitePool,
        profile_registry: &'a dyn AgentProfileResolver,
        event_bus: &'a SessionEventBus,
        tool_registry: &'a ToolRegistry,
    ) -> Self {
        Self {
            pool,
            profile_registry,
            event_bus,
            tool_registry,
            mcp_servers: HashMap::new(),
        }
    }

    /// 设置 MCP 服务器状态（用于工具健康检查）
    pub fn with_mcp_servers(mut self, servers: HashMap<String, McpServerRecord>) -> Self {
        self.mcp_servers = servers;
        self
    }

    /// 执行非流式请求
    pub async fn execute(
        &self,
        request: &ExecutionRequest,
        assistant_message_id: Option<String>,
    ) -> Result<(String, ExecutionArtifacts), AppError> {
        let session_id = request
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // 1. 加载 profile
        let profile = self
            .profile_registry
            .get_profile(&request.agent_profile_id)
            .map_err(|e| e.to_app_error())?;

        // 2. 路由选择模型
        let config = LlmProviderService::get_active_config(self.pool).await?;
        let model_selection = self.determine_model_selection(request, &profile);
        let selected_model = ModelRoutingService::select_model(
            &config,
            model_selection.to_routing_capability(),
            None,
            true,
        )
        .map_err(|e| e.to_app_error())?;

        // 3. 构建工具视图
        let tool_view = ToolExposureService::build_session_tool_view(
            &profile,
            self.tool_registry,
            &self.mcp_servers,
        )
        .map_err(|e| e.to_app_error())?;

        let tool_summary = tool_view
            .iter()
            .map(|t| format!("{}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        // 【WP-AI-BIZ-002】Agentic Search 阶段
        let mut evidence_items = Vec::new();
        let mut search_summary_json = None;

        if request.use_agentic_search {
            // 执行搜索阶段
            let search_orchestrator = AgenticSearchOrchestrator::new();
            let workspace_path =
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join(".pureworker/workspace");

            match search_orchestrator
                .search_stage(
                    self.pool,
                    &workspace_path,
                    crate::models::agentic_search::AgenticSearchInput {
                        query: request.user_input.clone(),
                        session_id: Some(session_id.clone()),
                        force_refresh: None,
                    },
                )
                .await
            {
                Ok(search_result) => {
                    evidence_items = search_result.evidence;
                    search_summary_json = Some(search_result.search_summary_json.clone());
                }
                Err(e) => {
                    eprintln!("[ExecutionOrchestrator] Agentic Search 失败: {}", e);
                }
            }
        }

        // 将证据转换为字符串列表
        let evidence: Vec<String> = evidence_items.into_iter().map(|e| e.content).collect();

        // 4. 组装提示词
        let templates_dir = runtime_templates_dir();
        let assembler = PromptAssemblerService::new(templates_dir);
        let assembled = assembler
            .assemble(
                request,
                &profile,
                &selected_model,
                &evidence, // 【WP-AI-BIZ-002】传入真实证据
                &tool_summary,
            )
            .map_err(|e| e.to_app_error())?;

        // 5. 发布开始事件 - 使用 assistant_message_id 或生成新的
        let message_id = assistant_message_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        self.publish_event(
            &session_id,
            SessionEvent::Start {
                version: SESSION_EVENT_VERSION,
                message_id: message_id.clone(),
            },
        )?;

        // 6. 创建 Provider Adapter 并调用
        let provider_config = LlmProviderService::create_provider_config(&config)?;
        let adapter = AdapterFactory::create(&provider_config);

        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: assembled.system_prompt,
                tool_call_id: None,
                tool_calls: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: assembled.user_prompt,
                tool_call_id: None,
                tool_calls: None,
            },
        ];

        // 构建工具定义
        let tools: Option<Vec<ToolDefinition>> = if tool_view.is_empty() {
            None
        } else {
            Some(
                tool_view
                    .iter()
                    .map(|t| ToolDefinition {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: serde_json::json!({"type": "object"}),
                    })
                    .collect(),
            )
        };

        // 7. 调用模型
        let content = adapter
            .chat(&selected_model.model_id, messages, tools)
            .await?;

        // 8. 发布完成事件
        self.publish_event(
            &session_id,
            SessionEvent::Complete {
                version: SESSION_EVENT_VERSION,
            },
        )?;

        // 【WP-AI-BIZ-003】持久化执行记录到 ExecutionStore
        // 注意：这里使用非阻塞方式调用，不等待结果
        let pool = self.pool.clone();
        let session_id_for_store = session_id.clone();
        let message_id_for_store = message_id.clone();
        let content_for_store = content.clone();
        let model_id_for_store = selected_model.model_id.clone();
        let search_summary_for_store = search_summary_json.clone();
        let entrypoint_for_store = request.entrypoint.clone();
        let agent_profile_id_for_store = request.agent_profile_id.clone();
        let user_input_for_store = request.user_input.clone();

        tokio::spawn(async move {
            // 1. 创建或获取会话
            let session_result = ExecutionStoreService::create_session(
                &pool,
                CreateExecutionSessionInput {
                    teacher_id: String::from("system"), // TODO: 从请求中获取真实 teacher_id
                    title: Some(format!("执行会话 {}", &session_id_for_store[..8])),
                    entrypoint: entrypoint_for_store.clone(),
                    agent_profile_id: agent_profile_id_for_store.clone(),
                },
            )
            .await;

            if let Ok(session) = session_result {
                // 2. 创建用户消息
                let _user_msg = ExecutionStoreService::create_message(
                    &pool,
                    CreateExecutionMessageInput {
                        session_id: session.id.clone(),
                        role: String::from("user"),
                        content: user_input_for_store,
                        tool_name: None,
                    },
                )
                .await;

                // 3. 创建助手消息
                if let Ok(_assistant_msg) = ExecutionStoreService::create_message(
                    &pool,
                    CreateExecutionMessageInput {
                        session_id: session.id.clone(),
                        role: String::from("assistant"),
                        content: content_for_store.clone(),
                        tool_name: None,
                    },
                )
                .await
                {
                    // 4. 创建执行记录
                    let _ = ExecutionStoreService::create_record(
                        &pool,
                        CreateExecutionRecordInput {
                            session_id: session_id_for_store.clone(),
                            execution_message_id: message_id_for_store.clone(),
                            entrypoint: entrypoint_for_store.clone(),
                            agent_profile_id: agent_profile_id_for_store.clone(),
                            model_id: model_id_for_store,
                            status: ExecutionStatus::Completed,
                            reasoning_summary: None,
                            search_summary_json: search_summary_for_store,
                            tool_calls_summary_json: None,
                            metadata_json: None,
                        },
                    )
                    .await;
                }
            }
        });

        Ok((
            session_id,
            ExecutionArtifacts {
                content,
                model_id: selected_model.model_id,
                reasoning_summary: None,
                search_summary_json: None,
                tool_calls_summary_json: None,
            },
        ))
    }

    /// 执行流式请求，返回 (session_id, 事件流)
    pub async fn execute_streaming(
        &self,
        request: &ExecutionRequest,
        assistant_message_id: Option<String>,
    ) -> Result<
        (
            String,
            Pin<Box<dyn Stream<Item = Result<SessionEvent, AppError>> + Send + '_>>,
        ),
        AppError,
    > {
        let session_id = request
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // 1. 加载 profile
        let profile = self
            .profile_registry
            .get_profile(&request.agent_profile_id)
            .map_err(|e| e.to_app_error())?;

        // 2. 路由选择模型
        let config = LlmProviderService::get_active_config(self.pool).await?;
        let model_selection = self.determine_model_selection(request, &profile);
        let selected_model = ModelRoutingService::select_model(
            &config,
            model_selection.to_routing_capability(),
            None,
            true,
        )
        .map_err(|e| e.to_app_error())?;

        // 3. 构建工具视图
        let tool_view = ToolExposureService::build_session_tool_view(
            &profile,
            self.tool_registry,
            &self.mcp_servers,
        )
        .map_err(|e| e.to_app_error())?;

        let tool_summary = tool_view
            .iter()
            .map(|t| format!("{}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        // 【WP-AI-BIZ-002】Agentic Search 阶段 - 仅在流式执行中集成
        let mut evidence_items = Vec::new();
        let mut search_summary_json = None;

        if request.use_agentic_search {
            // 发布思考状态事件 - 开始搜索
            let _ = self.publish_event(
                &session_id,
                SessionEvent::ThinkingStatus {
                    version: SESSION_EVENT_VERSION,
                    stage: String::from("searching"),
                    description: String::from("正在检索相关证据..."),
                },
            );

            // 执行搜索阶段
            let search_orchestrator = AgenticSearchOrchestrator::new();
            let workspace_path =
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join(".pureworker/workspace");

            match search_orchestrator
                .search_stage(
                    self.pool,
                    &workspace_path,
                    crate::models::agentic_search::AgenticSearchInput {
                        query: request.user_input.clone(),
                        session_id: Some(session_id.clone()),
                        force_refresh: None,
                    },
                )
                .await
            {
                Ok(search_result) => {
                    evidence_items = search_result.evidence;
                    search_summary_json = Some(search_result.search_summary_json.clone());

                    // 发布搜索摘要事件
                    let sources: Vec<String> = evidence_items
                        .iter()
                        .map(|e| e.source_table.clone())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect();

                    let _ = self.publish_event(
                        &session_id,
                        SessionEvent::SearchSummary {
                            version: SESSION_EVENT_VERSION,
                            sources,
                            evidence_count: evidence_items.len(),
                        },
                    );

                    // 发布推理摘要事件
                    let _ = self.publish_event(
                        &session_id,
                        SessionEvent::Reasoning {
                            version: SESSION_EVENT_VERSION,
                            summary: search_result.reasoning_summary,
                        },
                    );
                }
                Err(e) => {
                    eprintln!("[ExecutionOrchestrator] Agentic Search 失败: {}", e);
                    // 搜索失败继续执行，只是无证据
                }
            }
        }

        // 将证据转换为字符串列表
        let evidence: Vec<String> = evidence_items.into_iter().map(|e| e.content).collect();

        // 4. 组装提示词
        let templates_dir = runtime_templates_dir();
        let assembler = PromptAssemblerService::new(templates_dir);
        let assembled = assembler
            .assemble(
                request,
                &profile,
                &selected_model,
                &evidence, // 【WP-AI-BIZ-002】传入真实证据
                &tool_summary,
            )
            .map_err(|e| e.to_app_error())?;

        // 5. 创建 Provider Adapter
        let provider_config = LlmProviderService::create_provider_config(&config)?;
        let adapter = AdapterFactory::create(&provider_config);

        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: assembled.system_prompt,
                tool_call_id: None,
                tool_calls: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: assembled.user_prompt,
                tool_call_id: None,
                tool_calls: None,
            },
        ];

        // 构建工具定义
        let tools: Option<Vec<ToolDefinition>> = if tool_view.is_empty() {
            None
        } else {
            Some(
                tool_view
                    .iter()
                    .map(|t| ToolDefinition {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: serde_json::json!({"type": "object"}),
                    })
                    .collect(),
            )
        };

        // 6. 获取流式响应
        let stream = adapter
            .chat_stream(&selected_model.model_id, messages, tools)
            .await?;

        // 7. 转换为 SessionEvent 流 - 使用 assistant_message_id
        let message_id = assistant_message_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let event_stream = self.create_event_stream(session_id.clone(), message_id, stream);

        Ok((session_id, Box::pin(event_stream)))
    }

    /// 创建事件流，将 provider 的字符流转换为 SessionEvent 流
    fn create_event_stream(
        &self,
        _session_id: String,
        message_id: String,
        provider_stream: impl Stream<Item = Result<String, AppError>> + Send + 'static,
    ) -> impl Stream<Item = Result<SessionEvent, AppError>> + Send + '_ {
        let start_event = SessionEvent::Start {
            version: SESSION_EVENT_VERSION,
            message_id,
        };

        let start_stream = futures::stream::once(async move { Ok(start_event) });

        let content_stream = provider_stream.map(move |result| match result {
            Ok(content) => Ok(SessionEvent::Chunk {
                version: SESSION_EVENT_VERSION,
                content,
            }),
            Err(e) => Err(e),
        });

        let complete_stream = futures::stream::once(async move {
            Ok(SessionEvent::Complete {
                version: SESSION_EVENT_VERSION,
            })
        });

        start_stream.chain(content_stream).chain(complete_stream)
    }

    /// 确定模型选择类型
    fn determine_model_selection(
        &self,
        request: &ExecutionRequest,
        profile: &crate::services::ai_orchestration::RuntimeAgentProfile,
    ) -> RuntimeModelSelection {
        let has_attachments = !request.attachments.is_empty();

        if has_attachments || profile.prefer_multimodal {
            RuntimeModelSelection::Vision
        } else {
            RuntimeModelSelection::Text
        }
    }

    /// 发布事件到总线
    fn publish_event(&self, session_id: &str, event: SessionEvent) -> Result<(), AppError> {
        self.event_bus
            .append(session_id, event)
            .map_err(|e| e.to_app_error())
    }
}

fn runtime_templates_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("packages")
        .join("prompt-templates")
}

/// 执行编排器构建器
pub struct ExecutionOrchestratorBuilder<'a> {
    pool: &'a SqlitePool,
    profile_registry: Option<&'a dyn AgentProfileResolver>,
    event_bus: Option<&'a SessionEventBus>,
    tool_registry: Option<&'a ToolRegistry>,
    mcp_servers: HashMap<String, McpServerRecord>,
}

impl<'a> ExecutionOrchestratorBuilder<'a> {
    /// 创建新的构建器
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self {
            pool,
            profile_registry: None,
            event_bus: None,
            tool_registry: None,
            mcp_servers: HashMap::new(),
        }
    }

    /// 设置 Profile 注册表
    pub fn with_profile_registry(mut self, registry: &'a dyn AgentProfileResolver) -> Self {
        self.profile_registry = Some(registry);
        self
    }

    /// 设置事件总线
    pub fn with_event_bus(mut self, event_bus: &'a SessionEventBus) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// 设置工具注册表
    pub fn with_tool_registry(mut self, registry: &'a ToolRegistry) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// 设置 MCP 服务器
    pub fn with_mcp_servers(mut self, servers: HashMap<String, McpServerRecord>) -> Self {
        self.mcp_servers = servers;
        self
    }

    /// 构建执行编排器
    pub fn build(self) -> Result<ExecutionOrchestrator<'a>, AppError> {
        let profile_registry = self
            .profile_registry
            .ok_or_else(|| AppError::InvalidInput(String::from("未提供 Profile 注册表")))?;
        let event_bus = self
            .event_bus
            .ok_or_else(|| AppError::InvalidInput(String::from("未提供事件总线")))?;
        let tool_registry = self
            .tool_registry
            .ok_or_else(|| AppError::InvalidInput(String::from("未提供工具注册表")))?;

        Ok(ExecutionOrchestrator {
            pool: self.pool,
            profile_registry,
            event_bus,
            tool_registry,
            mcp_servers: self.mcp_servers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 验证 RuntimeModelSelection 到 RoutingCapability 的转换
    #[test]
    fn test_model_selection_to_capability() {
        assert_eq!(
            RuntimeModelSelection::Text.to_routing_capability(),
            RoutingCapability::Text
        );
        assert_eq!(
            RuntimeModelSelection::Vision.to_routing_capability(),
            RoutingCapability::Vision
        );
        assert_eq!(
            RuntimeModelSelection::Tool.to_routing_capability(),
            RoutingCapability::Tool
        );
        assert_eq!(
            RuntimeModelSelection::Reasoning.to_routing_capability(),
            RoutingCapability::Reasoning
        );
    }

    /// 验证 ExecutionArtifacts 默认值
    #[test]
    fn test_execution_artifacts_default() {
        let artifacts = ExecutionArtifacts::default();
        assert!(artifacts.content.is_empty());
        assert!(artifacts.model_id.is_empty());
        assert!(artifacts.reasoning_summary.is_none());
    }
}

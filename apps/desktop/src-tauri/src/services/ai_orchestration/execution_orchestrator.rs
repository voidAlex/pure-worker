//! 执行编排器
//!
//! 统一执行主链的核心编排服务，协调 profile 加载、模型路由、提示词组装、工具暴露和流式输出。
//! 支持作业批改统一编排：OCR 阶段 + LLM 批改阶段的一体化执行。

use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;

use futures::stream::{Stream, StreamExt};
use rig::completion::ToolDefinition;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::assignment_grading::{AssignmentAsset, AssignmentOcrResult};
use crate::models::execution::{ExecutionRequest, SessionEvent, SESSION_EVENT_VERSION};
use crate::models::mcp_server::McpServerRecord;
use crate::services::ai_orchestration::agent_profile_registry::AgentProfileRegistry;
use crate::services::ai_orchestration::model_routing::{ModelRoutingService, RoutingCapability};
use crate::services::ai_orchestration::prompt_assembler::PromptAssemblerService;
use crate::services::ai_orchestration::session_event_bus::SessionEventBus;
use crate::services::ai_orchestration::tool_exposure::ToolExposureService;
use crate::services::ai_orchestration::AgentProfileResolver;
use crate::services::llm_provider::LlmProviderService;
use crate::services::multimodal_grading::MultimodalGradingService;
use crate::services::ocr::OcrService;
use crate::services::provider_adapter::{AdapterFactory, ChatMessage, ChatRole};
use crate::services::tool_registry::{get_registry, ToolRegistry};

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

/// 批改管道阶段
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GradingStage {
    /// OCR 识别阶段
    Ocr,
    /// LLM 多模态批改阶段
    LlmGrading,
    /// 结果融合阶段
    Fusion,
}

/// 批改管道配置
#[derive(Debug, Clone)]
pub struct GradingPipelineConfig {
    /// 是否启用 OCR 阶段
    pub enable_ocr: bool,
    /// 是否启用 LLM 批改阶段
    pub enable_llm_grading: bool,
    /// 是否启用结果融合
    pub enable_fusion: bool,
    /// 标准答案 JSON
    pub answer_key_json: Option<String>,
    /// 评分规则 JSON
    pub scoring_rules_json: Option<String>,
    /// 工作区路径（用于 OCR 预处理）
    pub workspace_path: PathBuf,
}

impl Default for GradingPipelineConfig {
    fn default() -> Self {
        Self {
            enable_ocr: true,
            enable_llm_grading: true,
            enable_fusion: true,
            answer_key_json: None,
            scoring_rules_json: None,
            workspace_path: PathBuf::new(),
        }
    }
}

/// 批改管道结果
#[derive(Debug, Clone, Default)]
pub struct GradingPipelineResult {
    /// OCR 结果列表
    pub ocr_results: Vec<AssignmentOcrResult>,
    /// LLM 批改结果列表
    pub llm_results: Vec<AssignmentOcrResult>,
    /// 融合后的结果列表
    pub fused_results: Vec<AssignmentOcrResult>,
    /// 执行的阶段
    pub executed_stages: Vec<GradingStage>,
    /// 是否成功完成
    pub success: bool,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
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
    /// 批改管道结果（仅在批改场景下填充）
    pub grading_result: Option<GradingPipelineResult>,
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
    ) -> Result<ExecutionArtifacts, AppError> {
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

        // 4. 组装提示词
        let templates_dir = runtime_templates_dir();
        let assembler = PromptAssemblerService::new(templates_dir);
        let assembled = assembler
            .assemble(
                request,
                &profile,
                &selected_model,
                &[], // evidence - will be filled by agentic search if enabled
                &tool_summary,
            )
            .map_err(|e| e.to_app_error())?;

        // 5. 发布开始事件
        let message_id = uuid::Uuid::new_v4().to_string();
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

        Ok(ExecutionArtifacts {
            content,
            model_id: selected_model.model_id,
            reasoning_summary: None,
            search_summary_json: None,
            tool_calls_summary_json: None,
            grading_result: None,
        })
    }

    /// 执行流式请求，返回事件流
    pub async fn execute_streaming(
        &self,
        request: &ExecutionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<SessionEvent, AppError>> + Send + '_>>, AppError>
    {
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

        // 4. 组装提示词
        let templates_dir = runtime_templates_dir();
        let assembler = PromptAssemblerService::new(templates_dir);
        let assembled = assembler
            .assemble(
                request,
                &profile,
                &selected_model,
                &[], // evidence
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

        // 7. 转换为 SessionEvent 流
        let message_id = uuid::Uuid::new_v4().to_string();
        let event_stream = self.create_event_stream(session_id, message_id, stream);

        Ok(Box::pin(event_stream))
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

    /// 执行统一的作业批改管道（OCR + LLM 批改 + 结果融合）
    ///
    /// 该方法将 OCR 识别、LLM 多模态批改和结果融合整合为单一编排流程，
    /// 提供统一的事件流和错误处理。
    ///
    /// # 参数
    /// - `asset`: 作业资产
    /// - `job_id`: 批改任务 ID
    /// - `config`: 批改管道配置
    ///
    /// # 返回
    /// 批改管道结果，包含各阶段执行结果
    pub async fn execute_grading_pipeline(
        &self,
        asset: &AssignmentAsset,
        job_id: &str,
        config: &GradingPipelineConfig,
    ) -> Result<GradingPipelineResult, AppError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut result = GradingPipelineResult::default();
        let asset_id = &asset.id;

        // 发布管道开始事件
        self.publish_event(
            &session_id,
            SessionEvent::Start {
                version: SESSION_EVENT_VERSION,
                message_id: format!("grading_pipeline_{}", asset_id),
            },
        )?;

        // Stage 1: OCR 识别
        if config.enable_ocr {
            self.publish_event(
                &session_id,
                SessionEvent::ThinkingStatus {
                    version: SESSION_EVENT_VERSION,
                    stage: "ocr".to_string(),
                    description: "正在进行 OCR 文字识别...".to_string(),
                },
            )?;

            match OcrService::run_ocr_pipeline(self.pool, asset_id, job_id, &config.workspace_path)
                .await
            {
                Ok(ocr_results) => {
                    result.ocr_results = ocr_results;
                    result.executed_stages.push(GradingStage::Ocr);
                }
                Err(e) => {
                    result.error_message = Some(format!("OCR 阶段失败: {}", e));
                    self.publish_event(
                        &session_id,
                        SessionEvent::Error {
                            version: SESSION_EVENT_VERSION,
                            message: result.error_message.clone().unwrap(),
                        },
                    )?;
                    return Ok(result);
                }
            }
        }

        // Stage 2: LLM 多模态批改
        if config.enable_llm_grading {
            self.publish_event(
                &session_id,
                SessionEvent::ThinkingStatus {
                    version: SESSION_EVENT_VERSION,
                    stage: "llm_grading".to_string(),
                    description: "正在进行 LLM 多模态批改...".to_string(),
                },
            )?;

            match MultimodalGradingService::run_enhanced_grading(
                self.pool,
                asset_id,
                job_id,
                config.answer_key_json.as_deref(),
                config.scoring_rules_json.as_deref(),
            )
            .await
            {
                Ok(llm_results) => {
                    result.llm_results = llm_results;
                    result.executed_stages.push(GradingStage::LlmGrading);
                }
                Err(e) => {
                    // LLM 批改失败不中断流程，记录警告但继续
                    eprintln!("LLM 批改阶段失败: {}", e);
                }
            }
        }

        // Stage 3: 结果融合（如果启用了 OCR 和 LLM）
        if config.enable_fusion && !result.ocr_results.is_empty() {
            self.publish_event(
                &session_id,
                SessionEvent::ThinkingStatus {
                    version: SESSION_EVENT_VERSION,
                    stage: "fusion".to_string(),
                    description: "正在进行结果融合...".to_string(),
                },
            )?;

            // 融合结果已经在 run_enhanced_grading 中完成
            // 这里只需查询最终结果
            match sqlx::query_as::<_, AssignmentOcrResult>(
                "SELECT * FROM assignment_ocr_result WHERE asset_id = ? AND job_id = ? AND is_deleted = 0",
            )
            .bind(asset_id)
            .bind(job_id)
            .fetch_all(self.pool)
            .await
            {
                Ok(fused) => {
                    result.fused_results = fused;
                    result.executed_stages.push(GradingStage::Fusion);
                }
                Err(e) => {
                    eprintln!("查询融合结果失败: {}", e);
                }
            }
        }

        result.success = !result.ocr_results.is_empty();

        // 发布完成事件
        self.publish_event(
            &session_id,
            SessionEvent::Complete {
                version: SESSION_EVENT_VERSION,
            },
        )?;

        Ok(result)
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

/// 便捷函数：执行统一的作业批改管道
///
/// 无需手动构建 ExecutionOrchestrator，直接调用此函数执行完整的批改流程。
///
/// # 参数
/// - `pool`: 数据库连接池
/// - `asset`: 作业资产
/// - `job_id`: 批改任务 ID
/// - `config`: 批改管道配置
///
/// # 返回
/// 批改管道结果
pub async fn run_grading_pipeline(
    pool: &SqlitePool,
    asset: &AssignmentAsset,
    job_id: &str,
    config: &GradingPipelineConfig,
) -> Result<GradingPipelineResult, AppError> {
    let event_bus = SessionEventBus::new();
    let profile_registry = AgentProfileRegistry::new_default();
    let orchestrator = ExecutionOrchestratorBuilder::new(pool)
        .with_profile_registry(&profile_registry)
        .with_event_bus(&event_bus)
        .with_tool_registry(get_registry())
        .build()?;

    orchestrator
        .execute_grading_pipeline(asset, job_id, config)
        .await
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
        assert!(artifacts.grading_result.is_none());
    }

    /// 验证 GradingPipelineConfig 默认值
    #[test]
    fn test_grading_pipeline_config_default() {
        let config = GradingPipelineConfig::default();
        assert!(config.enable_ocr);
        assert!(config.enable_llm_grading);
        assert!(config.enable_fusion);
        assert!(config.answer_key_json.is_none());
        assert!(config.scoring_rules_json.is_none());
        assert!(config.workspace_path.as_os_str().is_empty());
    }

    /// 验证 GradingStage 枚举变体
    #[test]
    fn test_grading_stage_variants() {
        let stages = vec![
            GradingStage::Ocr,
            GradingStage::LlmGrading,
            GradingStage::Fusion,
        ];
        assert_eq!(stages.len(), 3);
        assert!(stages.contains(&GradingStage::Ocr));
        assert!(stages.contains(&GradingStage::LlmGrading));
        assert!(stages.contains(&GradingStage::Fusion));
    }

    /// 验证 GradingPipelineResult 默认状态
    #[test]
    fn test_grading_pipeline_result_default() {
        let result = GradingPipelineResult::default();
        assert!(result.ocr_results.is_empty());
        assert!(result.llm_results.is_empty());
        assert!(result.fused_results.is_empty());
        assert!(result.executed_stages.is_empty());
        assert!(!result.success);
        assert!(result.error_message.is_none());
    }

    /// 验证批改管道结果成功状态
    #[test]
    fn test_grading_pipeline_result_success() {
        let mut result = GradingPipelineResult::default();
        result.success = true;
        result.executed_stages.push(GradingStage::Ocr);
        result.executed_stages.push(GradingStage::LlmGrading);

        assert!(result.success);
        assert_eq!(result.executed_stages.len(), 2);
        assert_eq!(result.executed_stages[0], GradingStage::Ocr);
        assert_eq!(result.executed_stages[1], GradingStage::LlmGrading);
    }

    /// 验证批改管道配置自定义
    #[test]
    fn test_grading_pipeline_config_custom() {
        let config = GradingPipelineConfig {
            enable_ocr: true,
            enable_llm_grading: false,
            enable_fusion: true,
            answer_key_json: Some(r#"{"1": "A"}"#.to_string()),
            scoring_rules_json: Some(r#"{"1": {"full_score": 10}}"#.to_string()),
            workspace_path: std::path::PathBuf::from("/tmp/workspace"),
        };

        assert!(config.enable_ocr);
        assert!(!config.enable_llm_grading);
        assert!(config.enable_fusion);
        assert_eq!(config.answer_key_json.as_ref().unwrap(), r#"{"1": "A"}"#);
        assert_eq!(
            config.scoring_rules_json.as_ref().unwrap(),
            r#"{"1": {"full_score": 10}}"#
        );
        assert_eq!(config.workspace_path.to_str().unwrap(), "/tmp/workspace");
    }

    /// 验证 GradingPipelineResult 错误状态
    #[test]
    fn test_grading_pipeline_result_with_error() {
        let mut result = GradingPipelineResult::default();
        result.error_message = Some("OCR 阶段失败: 模型未找到".to_string());
        result.executed_stages.push(GradingStage::Ocr);

        assert!(!result.success);
        assert_eq!(
            result.error_message.as_ref().unwrap(),
            "OCR 阶段失败: 模型未找到"
        );
        assert_eq!(result.executed_stages.len(), 1);
    }
}

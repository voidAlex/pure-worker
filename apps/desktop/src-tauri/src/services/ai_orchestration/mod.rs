//! AI 编排运行时模块
//!
//! 提供统一执行主链的核心抽象，包含：
//! - 错误边界与 `AppError` 映射
//! - 执行阶段 trait
//! - 运行时关键能力（profile、prompt、model routing、store、event bus）trait 定义
//! - 执行编排器与请求工厂

pub mod agent_profile_registry;
pub mod error;
pub mod execution_orchestrator;
pub mod execution_request_factory;
pub mod execution_stage;
pub mod execution_store;
pub mod model_routing;
pub mod prompt_assembler;
pub mod provider_catalog;
pub mod session_event_bus;
pub mod tool_exposure;

use self::agent_profile_registry::OutputProtocol;
pub use agent_profile_registry::AgentProfileRegistry;
pub use error::{OrchestrationError, OrchestrationResult};
pub use execution_orchestrator::{
    ExecutionArtifacts, ExecutionOrchestrator, ExecutionOrchestratorBuilder, RuntimeModelSelection,
};
pub use execution_stage::{ExecutionStage, ExecutionStageContext, ExecutionStageOutput};
pub use model_routing::{RoutingCapability, RoutingTrace, SelectedModel};
pub use prompt_assembler::AssembledPrompt;

use crate::models::execution::{
    ExecutionEntrypoint, ExecutionRequest, ExecutionStatus, SessionEvent,
};
use crate::services::unified_tool::ToolRiskLevel;

/// Agent Profile 读取能力
pub trait AgentProfileResolver: Send + Sync {
    /// 获取运行时 profile
    fn get_profile(&self, profile_id: &str) -> OrchestrationResult<RuntimeAgentProfile>;
}

/// Prompt 组装能力
pub trait PromptAssembler: Send + Sync {
    /// 组装最终 prompt
    fn assemble(
        &self,
        request: &ExecutionRequest,
        profile: &RuntimeAgentProfile,
        selected_model: &model_routing::SelectedModel,
        evidence: &[String],
        tool_summary: &str,
    ) -> OrchestrationResult<AssembledPrompt>;
}

/// 模型路由能力
pub trait ModelRouter: Send + Sync {
    /// 按请求与 profile 选择模型
    fn select_model(
        &self,
        request: &ExecutionRequest,
        profile: &RuntimeAgentProfile,
    ) -> OrchestrationResult<SelectedModel>;
}

/// 执行存储能力
pub trait ExecutionStore: Send + Sync {
    /// 持久化执行生命周期事件
    fn persist_events(
        &self,
        request: &ExecutionRequest,
        status: ExecutionStatus,
        events: &[SessionEvent],
    ) -> OrchestrationResult<()>;
}

/// 会话事件总线能力
pub trait SessionEventPublisher: Send + Sync {
    /// 发布事件到前端订阅侧
    fn publish(&self, session_id: &str, events: &[SessionEvent]) -> OrchestrationResult<()>;
}

/// 运行时 Agent Profile 精简快照
#[derive(Debug, Clone)]
pub struct RuntimeAgentProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entrypoint: ExecutionEntrypoint,
    pub tool_allowlist: Vec<String>,
    pub tool_denylist: Vec<String>,
    pub output_protocol: OutputProtocol,
    pub max_tool_risk: ToolRiskLevel,
    pub requires_agentic_search: bool,
    pub prefer_multimodal: bool,
}

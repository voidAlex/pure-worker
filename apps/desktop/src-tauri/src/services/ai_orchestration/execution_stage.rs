//! 执行阶段抽象
//!
//! 定义统一执行主链中的阶段接口，便于在同一编排管线中串联搜索、推理、工具前置等步骤。

use async_trait::async_trait;

use crate::models::execution::{ExecutionRequest, SessionEvent};

use super::error::OrchestrationResult;

/// 执行阶段上下文
#[derive(Debug)]
pub struct ExecutionStageContext {
    /// 当前执行请求
    pub request: ExecutionRequest,
    /// 选中的模型 ID
    pub model_id: String,
    /// 当前会话 ID
    pub session_id: String,
    /// 阶段间共享证据摘要
    pub evidence: Vec<String>,
}

/// 执行阶段输出
#[derive(Debug, Clone, Default)]
pub struct ExecutionStageOutput {
    /// 当前阶段新增事件
    pub emitted_events: Vec<SessionEvent>,
    /// 当前阶段新增证据
    pub appended_evidence: Vec<String>,
}

/// 统一执行阶段 trait
#[async_trait]
pub trait ExecutionStage: Send + Sync {
    /// 阶段名称（用于日志/链路追踪）
    fn stage_name(&self) -> &'static str;

    /// 执行阶段逻辑
    async fn run(
        &self,
        context: &mut ExecutionStageContext,
    ) -> OrchestrationResult<ExecutionStageOutput>;
}

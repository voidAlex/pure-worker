//! 统一工具协议模块
//!
//! 定义所有工具（内置技能、Python 技能、MCP 工具）必须遵循的统一协议，
//! 包括统一 trait、返回结构、审计信息和风险等级。
//!
//! 技术方案要求：
//! - 统一 JSON Schema（name/description/inputSchema/outputSchema）
//! - 统一返回结构（success/data/error/degraded_to）
//! - 统一审计字段（tool_name、invoke_id、risk_level、duration_ms）

use chrono::Utc;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use crate::error::AppError;

/// 工具输入类型别名（JSON 值）。
pub type ToolInput = serde_json::Value;

/// 工具输出类型别名（JSON 值）。
pub type ToolOutput = serde_json::Value;

/// 统一工具协议 trait（对象安全版本）。
///
/// 所有工具（内置技能、Python 技能、MCP 工具）必须实现此 trait，
/// 以确保统一的调用接口、Schema 声明和审计能力。
///
/// 使用 `Pin<Box<dyn Future>>` 返回类型以支持 trait 对象（`dyn UnifiedTool`）。
pub trait UnifiedTool: Send + Sync {
    /// 获取工具名称。
    fn name(&self) -> &str;

    /// 获取工具描述。
    fn description(&self) -> &str;

    /// 获取输入参数的 JSON Schema。
    fn input_schema(&self) -> serde_json::Value;

    /// 获取输出参数的 JSON Schema。
    fn output_schema(&self) -> serde_json::Value;

    /// 获取工具的风险等级。
    fn risk_level(&self) -> ToolRiskLevel;

    /// 执行工具调用。
    ///
    /// 接收 JSON 格式的输入参数和外部分配的 `invoke_id`，返回统一的 `ToolResult` 结果。
    /// `invoke_id` 由执行引擎统一生成并传入，确保全链路审计可追踪。
    /// 返回 `Pin<Box<dyn Future>>` 以保持 trait 对象安全。
    fn invoke(
        &self,
        input: serde_json::Value,
        invoke_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, AppError>> + Send + '_>>;
}

/// 统一工具返回结构。
///
/// 所有工具调用的标准返回格式，包含执行结果、错误信息、降级标识和审计数据。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ToolResult {
    /// 执行是否成功。
    pub success: bool,
    /// 成功时的返回数据（JSON 格式）。
    pub data: Option<serde_json::Value>,
    /// 失败时的错误描述。
    pub error: Option<String>,
    /// 降级回退标识（当原始工具不可用、自动降级到备选方案时填写）。
    pub degraded_to: Option<String>,
    /// 审计信息。
    pub audit: ToolAuditInfo,
}

/// 工具审计信息。
///
/// 记录每次工具调用的追踪数据，用于审计日志和性能监控。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ToolAuditInfo {
    /// 工具名称。
    pub tool_name: String,
    /// 调用唯一标识。
    pub invoke_id: String,
    /// 风险等级（"low"/"medium"/"high"）。
    pub risk_level: String,
    /// 执行耗时（毫秒）。
    pub duration_ms: u64,
    /// 调用时间戳（ISO 8601 格式）。
    pub timestamp: String,
}

/// 工具风险等级枚举。
///
/// 用于标识工具操作的风险程度，影响审计记录和审批流程。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum ToolRiskLevel {
    /// 低风险：只读操作、计算类操作。
    Low,
    /// 中风险：文件读取、数据查询。
    Medium,
    /// 高风险：文件写入、外部调用、批量修改。
    High,
}

impl ToolRiskLevel {
    /// 转换为字符串表示。
    pub fn as_str(&self) -> &str {
        match self {
            ToolRiskLevel::Low => "low",
            ToolRiskLevel::Medium => "medium",
            ToolRiskLevel::High => "high",
        }
    }
}

impl fmt::Display for ToolRiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 创建成功的工具执行结果。
///
/// 自动填充审计信息中的时间戳。
///
/// # 参数
/// - `tool_name`: 工具名称
/// - `invoke_id`: 调用唯一标识
/// - `risk_level`: 风险等级
/// - `duration_ms`: 执行耗时（毫秒）
/// - `data`: 返回数据（JSON 格式）
pub fn create_success_result(
    tool_name: &str,
    invoke_id: &str,
    risk_level: ToolRiskLevel,
    duration_ms: u64,
    data: serde_json::Value,
) -> ToolResult {
    ToolResult {
        success: true,
        data: Some(data),
        error: None,
        degraded_to: None,
        audit: ToolAuditInfo {
            tool_name: tool_name.to_string(),
            invoke_id: invoke_id.to_string(),
            risk_level: risk_level.as_str().to_string(),
            duration_ms,
            timestamp: Utc::now().to_rfc3339(),
        },
    }
}

/// 创建失败的工具执行结果。
///
/// 自动填充审计信息中的时间戳。
///
/// # 参数
/// - `tool_name`: 工具名称
/// - `invoke_id`: 调用唯一标识
/// - `risk_level`: 风险等级
/// - `duration_ms`: 执行耗时（毫秒）
/// - `error`: 错误描述信息
pub fn create_error_result(
    tool_name: &str,
    invoke_id: &str,
    risk_level: ToolRiskLevel,
    duration_ms: u64,
    error: String,
) -> ToolResult {
    ToolResult {
        success: false,
        data: None,
        error: Some(error),
        degraded_to: None,
        audit: ToolAuditInfo {
            tool_name: tool_name.to_string(),
            invoke_id: invoke_id.to_string(),
            risk_level: risk_level.as_str().to_string(),
            duration_ms,
            timestamp: Utc::now().to_rfc3339(),
        },
    }
}

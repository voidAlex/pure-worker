//! MCP 工具适配器（WP-AI-007）
//!
//! 将 MCP 工具适配为 UnifiedTool trait，使其可以集成到 Tool Registry。

use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

use crate::error::AppError;
use crate::models::mcp_server::McpServerRecord;
use crate::services::mcp_runtime::{McpClient, McpRuntime, McpTool};
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel, UnifiedTool,
};

/// MCP 工具适配器
///
/// 包装 MCP 工具，使其符合 UnifiedTool trait。
pub struct McpToolAdapter {
    server: McpServerRecord,
    tool: McpTool,
    risk_level: ToolRiskLevel,
}

impl McpToolAdapter {
    /// 创建新的 MCP 工具适配器
    pub fn new(server: McpServerRecord, tool: McpTool) -> Self {
        // 根据权限范围确定风险等级
        let risk_level = match server.permission_scope.as_deref() {
            Some("read_only") => ToolRiskLevel::Low,
            Some("file_system") => ToolRiskLevel::High,
            Some("network") => ToolRiskLevel::High,
            Some("system") => ToolRiskLevel::High,
            _ => ToolRiskLevel::Medium,
        };

        Self {
            server,
            tool,
            risk_level,
        }
    }

    /// 获取服务器 ID
    pub fn server_id(&self) -> &str {
        &self.server.id
    }

    /// 获取原始 MCP 工具定义
    pub fn mcp_tool(&self) -> &McpTool {
        &self.tool
    }
}

impl UnifiedTool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.tool.name
    }

    fn description(&self) -> &str {
        self.tool.description.as_deref().unwrap_or("MCP 工具")
    }

    fn input_schema(&self) -> serde_json::Value {
        self.tool.input_schema.clone()
    }

    fn output_schema(&self) -> serde_json::Value {
        // MCP 工具输出格式不固定，返回通用对象 schema
        serde_json::json!({
            "type": "object",
            "description": "MCP 工具输出（格式取决于具体工具）"
        })
    }

    fn risk_level(&self) -> ToolRiskLevel {
        self.risk_level
    }

    fn invoke(
        &self,
        input: serde_json::Value,
        invoke_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, AppError>> + Send + '_>> {
        let server = self.server.clone();
        let tool_name = self.tool.name.clone();
        let invoke_id = invoke_id.to_string();
        let risk_level = self.risk_level;

        Box::pin(async move {
            let start = Instant::now();

            match McpClient::connect(server).await {
                Ok(mut client) => match client.call_tool(&tool_name, input).await {
                    Ok(result) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        Ok(create_success_result(
                            &tool_name,
                            &invoke_id,
                            risk_level,
                            duration_ms,
                            result,
                        ))
                    }
                    Err(e) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        Ok(create_error_result(
                            &tool_name,
                            &invoke_id,
                            risk_level,
                            duration_ms,
                            e.to_string(),
                        ))
                    }
                },
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    Ok(create_error_result(
                        &tool_name,
                        &invoke_id,
                        risk_level,
                        duration_ms,
                        format!("连接 MCP 服务器失败: {}", e),
                    ))
                }
            }
        })
    }
}

/// 从数据库加载所有 MCP 工具并注册到 Tool Registry
pub async fn register_mcp_tools(
    pool: &sqlx::SqlitePool,
    registry: &crate::services::tool_registry::ToolRegistry,
) -> Result<usize, AppError> {
    let servers_with_tools = McpRuntime::load_all_tools(pool).await?;
    let mut count = 0;

    for (server, tools) in servers_with_tools {
        for tool in tools {
            let adapter = McpToolAdapter::new(server.clone(), tool);
            registry.register(
                std::sync::Arc::new(adapter),
                crate::services::tool_registry::ToolCategory::Mcp,
                crate::services::tool_registry::ToolSource::Mcp {
                    server_id: server.id.clone(),
                },
            )?;
            count += 1;
        }
    }

    Ok(count)
}

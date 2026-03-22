//! 会话级工具暴露服务
//!
//! 负责依据 profile allowlist、denylist、风险上限、入口点能力和 MCP 健康状态生成可见工具视图。

use std::collections::HashMap;

use crate::models::execution::ExecutionEntrypoint;
use crate::models::mcp_server::McpServerRecord;
use crate::services::tool_registry::{ToolRegistry, ToolSource};
use crate::services::unified_tool::ToolRiskLevel;

use super::{OrchestrationResult, RuntimeAgentProfile};

/// 会话级工具视图
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionToolView {
    pub name: String,
    pub description: String,
    pub risk_level: ToolRiskLevel,
    pub source_label: String,
}

/// 工具暴露服务
pub struct ToolExposureService;

impl ToolExposureService {
    pub fn build_session_tool_view(
        profile: &RuntimeAgentProfile,
        registry: &ToolRegistry,
        mcp_servers: &HashMap<String, McpServerRecord>,
    ) -> OrchestrationResult<Vec<SessionToolView>> {
        let mut visible_tools = registry
            .list_all()
            .into_iter()
            .filter(|tool| allow_by_profile(profile, &tool.name))
            .filter(|tool| tool.risk_level.rank() <= profile.max_tool_risk.rank())
            .filter(|tool| allow_by_entrypoint(profile.entrypoint.clone(), &tool.name))
            .filter(|tool| allow_mcp_health(tool.source.clone(), mcp_servers))
            .map(|tool| SessionToolView {
                name: tool.name,
                description: tool.description,
                risk_level: tool.risk_level,
                source_label: source_label(&tool.source),
            })
            .collect::<Vec<SessionToolView>>();

        visible_tools.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(visible_tools)
    }
}

trait ToolRiskLevelExt {
    fn rank(self) -> u8;
}

impl ToolRiskLevelExt for ToolRiskLevel {
    fn rank(self) -> u8 {
        match self {
            ToolRiskLevel::Low => 0,
            ToolRiskLevel::Medium => 1,
            ToolRiskLevel::High => 2,
        }
    }
}

fn allow_by_profile(profile: &RuntimeAgentProfile, tool_name: &str) -> bool {
    let allowlisted = profile.tool_allowlist.is_empty()
        || profile.tool_allowlist.iter().any(|item| item == tool_name);
    let denied = profile.tool_denylist.iter().any(|item| item == tool_name);
    allowlisted && !denied
}

fn allow_by_entrypoint(entrypoint: ExecutionEntrypoint, tool_name: &str) -> bool {
    match entrypoint {
        ExecutionEntrypoint::Search => tool_name.starts_with("search."),
        _ => true,
    }
}

fn allow_mcp_health(source: ToolSource, mcp_servers: &HashMap<String, McpServerRecord>) -> bool {
    match source {
        ToolSource::Mcp { server_id } => mcp_servers
            .get(&server_id)
            .map(|server| server.enabled == 1 && server.health_status == "healthy")
            .unwrap_or(false),
        _ => true,
    }
}

fn source_label(source: &ToolSource) -> String {
    match source {
        ToolSource::Builtin => String::from("builtin"),
        ToolSource::Skill { skill_id } => format!("skill:{skill_id}"),
        ToolSource::Mcp { server_id } => format!("mcp:{server_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::execution::ExecutionEntrypoint;
    use crate::services::ai_orchestration::agent_profile_registry::OutputProtocol;
    use crate::services::tool_registry::{ToolCategory, ToolSource};
    use crate::services::unified_tool::{ToolResult, UnifiedTool};
    use serde_json::json;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    struct FakeTool {
        name: &'static str,
        description: &'static str,
        risk: ToolRiskLevel,
    }

    impl UnifiedTool for FakeTool {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            self.description
        }
        fn input_schema(&self) -> serde_json::Value {
            json!({"type": "object"})
        }
        fn output_schema(&self) -> serde_json::Value {
            json!({"type": "object"})
        }
        fn risk_level(&self) -> ToolRiskLevel {
            self.risk
        }
        fn invoke(
            &self,
            _input: serde_json::Value,
            _invoke_id: &str,
        ) -> Pin<Box<dyn Future<Output = Result<ToolResult, crate::error::AppError>> + Send + '_>>
        {
            Box::pin(async { unreachable!("tests do not invoke tools") })
        }
    }

    fn test_profile(entrypoint: ExecutionEntrypoint) -> RuntimeAgentProfile {
        RuntimeAgentProfile {
            id: String::from("test.profile"),
            name: String::from("测试 profile"),
            description: String::from("用于测试工具过滤"),
            entrypoint,
            tool_allowlist: vec![
                String::from("search.student"),
                String::from("skill.note"),
                String::from("mcp.lookup"),
            ],
            tool_denylist: vec![String::from("skill.note")],
            output_protocol: OutputProtocol::Markdown,
            max_tool_risk: ToolRiskLevel::Medium,
            requires_agentic_search: false,
            prefer_multimodal: false,
        }
    }

    fn healthy_mcp_server() -> McpServerRecord {
        McpServerRecord {
            id: String::from("mcp-1"),
            name: String::from("mcp-1"),
            transport: String::from("stdio"),
            command: None,
            args_json: None,
            env_json: None,
            permission_scope: Some(String::from("read_only")),
            enabled: 1,
            is_deleted: 0,
            created_at: String::from("2026-03-22T00:00:00Z"),
            display_name: Some(String::from("查询 MCP")),
            description: None,
            health_status: String::from("healthy"),
            last_health_check: None,
            updated_at: Some(String::from("2026-03-22T00:00:00Z")),
        }
    }

    fn register_test_tools(registry: &ToolRegistry) {
        registry
            .register(
                Arc::new(FakeTool {
                    name: "search.student",
                    description: "学生搜索",
                    risk: ToolRiskLevel::Low,
                }),
                ToolCategory::Builtin,
                ToolSource::Builtin,
            )
            .unwrap();
        registry
            .register(
                Arc::new(FakeTool {
                    name: "skill.note",
                    description: "记录技能",
                    risk: ToolRiskLevel::Medium,
                }),
                ToolCategory::Skill,
                ToolSource::Skill {
                    skill_id: String::from("skill-1"),
                },
            )
            .unwrap();
        registry
            .register(
                Arc::new(FakeTool {
                    name: "mcp.lookup",
                    description: "MCP 查询",
                    risk: ToolRiskLevel::Low,
                }),
                ToolCategory::Mcp,
                ToolSource::Mcp {
                    server_id: String::from("mcp-1"),
                },
            )
            .unwrap();
        registry
            .register(
                Arc::new(FakeTool {
                    name: "filesystem.write",
                    description: "写文件",
                    risk: ToolRiskLevel::High,
                }),
                ToolCategory::Builtin,
                ToolSource::Builtin,
            )
            .unwrap();
    }

    /// 验证 allowlist 与 denylist 同时生效
    #[test]
    fn test_allowlist_and_denylist_filtering() {
        let registry = ToolRegistry::new();
        register_test_tools(&registry);
        let profile = test_profile(ExecutionEntrypoint::Chat);
        let servers = HashMap::from([(String::from("mcp-1"), healthy_mcp_server())]);

        let result = ToolExposureService::build_session_tool_view(&profile, &registry, &servers);
        let tools = result.expect("tool exposure should succeed");

        assert!(tools.iter().any(|tool| tool.name == "search.student"));
        assert!(!tools.iter().any(|tool| tool.name == "skill.note"));
    }

    /// 验证风险上限过滤高风险工具
    #[test]
    fn test_risk_ceiling_filtering() {
        let registry = ToolRegistry::new();
        register_test_tools(&registry);
        let mut profile = test_profile(ExecutionEntrypoint::Chat);
        profile
            .tool_allowlist
            .push(String::from("filesystem.write"));
        let servers = HashMap::from([(String::from("mcp-1"), healthy_mcp_server())]);

        let result = ToolExposureService::build_session_tool_view(&profile, &registry, &servers);
        let tools = result.expect("tool exposure should succeed");

        assert!(!tools.iter().any(|tool| tool.name == "filesystem.write"));
    }

    /// 验证 search 入口点只保留搜索能力工具
    #[test]
    fn test_entrypoint_capability_filtering() {
        let registry = ToolRegistry::new();
        register_test_tools(&registry);
        let mut profile = test_profile(ExecutionEntrypoint::Search);
        profile.tool_allowlist = vec![String::from("search.student"), String::from("mcp.lookup")];
        let servers = HashMap::from([(String::from("mcp-1"), healthy_mcp_server())]);

        let result = ToolExposureService::build_session_tool_view(&profile, &registry, &servers);
        let tools = result.expect("tool exposure should succeed");

        assert!(tools.iter().all(|tool| tool.name.starts_with("search.")));
    }

    /// 验证 MCP 未启用或不健康时不暴露
    #[test]
    fn test_mcp_enabled_and_health_filtering() {
        let registry = ToolRegistry::new();
        register_test_tools(&registry);
        let profile = test_profile(ExecutionEntrypoint::Chat);
        let mut unhealthy = healthy_mcp_server();
        unhealthy.enabled = 0;
        unhealthy.health_status = String::from("unhealthy");
        let servers = HashMap::from([(String::from("mcp-1"), unhealthy)]);

        let result = ToolExposureService::build_session_tool_view(&profile, &registry, &servers);
        let tools = result.expect("tool exposure should succeed");

        assert!(!tools.iter().any(|tool| tool.name == "mcp.lookup"));
    }
}

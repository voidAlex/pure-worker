//! 运行时 Agent Profile 注册表
//!
//! 提供统一 profile 装载与查询能力，作为后续持久化 profile 的稳定入口。

use std::collections::HashMap;

use crate::models::execution::ExecutionEntrypoint;
use crate::services::unified_tool::ToolRiskLevel;

use super::{
    error::{OrchestrationError, OrchestrationResult},
    AgentProfileResolver, RuntimeAgentProfile,
};

/// 输出协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputProtocol {
    /// 常规文本输出
    Markdown,
    /// 严格 JSON 输出
    Json,
}

impl OutputProtocol {
    /// 输出协议标识
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
        }
    }
}

/// 运行时 profile 定义
#[derive(Debug, Clone)]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entrypoint: ExecutionEntrypoint,
    pub tool_allowlist: Vec<String>,
    pub tool_denylist: Vec<String>,
    pub max_tool_risk: ToolRiskLevel,
    pub output_protocol: OutputProtocol,
    pub requires_agentic_search: bool,
    pub prefer_multimodal: bool,
}

/// Agent Profile 注册中心
pub struct AgentProfileRegistry {
    profiles: HashMap<String, AgentProfile>,
}

/// Agent Profile 来源抽象
pub trait AgentProfileSource: Send + Sync {
    fn load_profiles(&self) -> Vec<AgentProfile>;
}

/// 静态内置 profile 来源
pub struct StaticAgentProfileSource {
    profiles: Vec<AgentProfile>,
}

impl StaticAgentProfileSource {
    pub fn builtin() -> Self {
        Self {
            profiles: builtin_profiles(),
        }
    }
}

impl AgentProfileSource for StaticAgentProfileSource {
    fn load_profiles(&self) -> Vec<AgentProfile> {
        self.profiles.clone()
    }
}

impl AgentProfileRegistry {
    /// 创建默认注册表（内置 profile）
    pub fn new_default() -> Self {
        Self::from_source(&StaticAgentProfileSource::builtin())
    }

    /// 从外部 profile 列表创建注册表
    pub fn from_profiles(profiles: Vec<AgentProfile>) -> Self {
        let profiles = profiles
            .into_iter()
            .map(|profile| (profile.id.clone(), profile))
            .collect();
        Self { profiles }
    }

    /// 查询指定 profile
    pub fn get_profile(&self, profile_id: &str) -> OrchestrationResult<&AgentProfile> {
        self.profiles.get(profile_id).ok_or_else(|| {
            OrchestrationError::ProfileNotFound(format!("未找到 Agent Profile：{}", profile_id))
        })
    }

    /// 从 profile 来源构建注册表
    pub fn from_source(source: &dyn AgentProfileSource) -> Self {
        Self::from_profiles(source.load_profiles())
    }

    /// 获取全部 profile
    pub fn list_profiles(&self) -> Vec<&AgentProfile> {
        self.profiles.values().collect()
    }

    /// 检查 profile 是否存在
    pub fn has_profile(&self, profile_id: &str) -> bool {
        self.profiles.contains_key(profile_id)
    }
}

impl AgentProfileResolver for AgentProfileRegistry {
    fn get_profile(&self, profile_id: &str) -> OrchestrationResult<RuntimeAgentProfile> {
        let profile = AgentProfileRegistry::get_profile(self, profile_id)?;
        Ok(RuntimeAgentProfile::from(profile))
    }
}

impl From<&AgentProfile> for RuntimeAgentProfile {
    fn from(profile: &AgentProfile) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            description: profile.description.clone(),
            entrypoint: profile.entrypoint.clone(),
            tool_allowlist: profile.tool_allowlist.clone(),
            tool_denylist: profile.tool_denylist.clone(),
            output_protocol: profile.output_protocol,
            max_tool_risk: profile.max_tool_risk,
            requires_agentic_search: profile.requires_agentic_search,
            prefer_multimodal: profile.prefer_multimodal,
        }
    }
}

fn builtin_profiles() -> Vec<AgentProfile> {
    vec![
        AgentProfile {
            id: String::from("chat.homeroom"),
            name: String::from("班主任对话"),
            description: String::from("用于班务场景的通用对话与建议生成"),
            entrypoint: ExecutionEntrypoint::Chat,
            tool_allowlist: vec![
                String::from("search.student"),
                String::from("search.memory"),
                String::from("classroom.list"),
            ],
            tool_denylist: vec![],
            max_tool_risk: ToolRiskLevel::Medium,
            output_protocol: OutputProtocol::Markdown,
            requires_agentic_search: false,
            prefer_multimodal: false,
        },
        AgentProfile {
            id: String::from("chat.grading"),
            name: String::from("批改对话"),
            description: String::from("用于作业与评语生成，优先多模态理解"),
            entrypoint: ExecutionEntrypoint::Grading,
            tool_allowlist: vec![String::from("ocr.image"), String::from("grading.rubric")],
            tool_denylist: vec![],
            max_tool_risk: ToolRiskLevel::Medium,
            output_protocol: OutputProtocol::Json,
            requires_agentic_search: false,
            prefer_multimodal: true,
        },
        AgentProfile {
            id: String::from("chat.communication"),
            name: String::from("家校沟通"),
            description: String::from("用于家校沟通草稿生成"),
            entrypoint: ExecutionEntrypoint::Communication,
            tool_allowlist: vec![
                String::from("search.student"),
                String::from("search.memory"),
            ],
            tool_denylist: vec![],
            max_tool_risk: ToolRiskLevel::Low,
            output_protocol: OutputProtocol::Markdown,
            requires_agentic_search: false,
            prefer_multimodal: false,
        },
        AgentProfile {
            id: String::from("generation.parent_communication"),
            name: String::from("家长沟通生成"),
            description: String::from("用于生成家长沟通草稿 JSON"),
            entrypoint: ExecutionEntrypoint::Communication,
            tool_allowlist: vec![
                String::from("search.memory"),
                String::from("search.student"),
            ],
            tool_denylist: vec![],
            max_tool_risk: ToolRiskLevel::Low,
            output_protocol: OutputProtocol::Json,
            requires_agentic_search: false,
            prefer_multimodal: false,
        },
        AgentProfile {
            id: String::from("generation.semester_comment"),
            name: String::from("学期评语生成"),
            description: String::from("用于生成学期评语 JSON 草稿"),
            entrypoint: ExecutionEntrypoint::Communication,
            tool_allowlist: vec![
                String::from("search.memory"),
                String::from("search.student"),
            ],
            tool_denylist: vec![],
            max_tool_risk: ToolRiskLevel::Low,
            output_protocol: OutputProtocol::Json,
            requires_agentic_search: false,
            prefer_multimodal: false,
        },
        AgentProfile {
            id: String::from("generation.activity_announcement"),
            name: String::from("活动公告生成"),
            description: String::from("用于生成活动公告 JSON 草稿"),
            entrypoint: ExecutionEntrypoint::Communication,
            tool_allowlist: vec![],
            tool_denylist: vec![],
            max_tool_risk: ToolRiskLevel::Low,
            output_protocol: OutputProtocol::Json,
            requires_agentic_search: false,
            prefer_multimodal: false,
        },
        AgentProfile {
            id: String::from("chat.ops"),
            name: String::from("运维助手"),
            description: String::from("用于系统配置建议与运维排障"),
            entrypoint: ExecutionEntrypoint::Chat,
            tool_allowlist: vec![String::from("settings.get"), String::from("health.check")],
            tool_denylist: vec![String::from("filesystem.delete")],
            max_tool_risk: ToolRiskLevel::Medium,
            output_protocol: OutputProtocol::Markdown,
            requires_agentic_search: true,
            prefer_multimodal: false,
        },
        AgentProfile {
            id: String::from("search.agentic"),
            name: String::from("Agentic 检索"),
            description: String::from("用于先检索再生成的证据增强流程"),
            entrypoint: ExecutionEntrypoint::Search,
            tool_allowlist: vec![
                String::from("search.student"),
                String::from("search.memory"),
            ],
            tool_denylist: vec![],
            max_tool_risk: ToolRiskLevel::Low,
            output_protocol: OutputProtocol::Markdown,
            requires_agentic_search: true,
            prefer_multimodal: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证内置 profile 全量加载
    #[test]
    fn test_load_builtin_profiles() {
        let registry = AgentProfileRegistry::new_default();

        for profile_id in [
            "chat.homeroom",
            "chat.grading",
            "chat.communication",
            "generation.parent_communication",
            "generation.semester_comment",
            "generation.activity_announcement",
            "chat.ops",
            "search.agentic",
        ] {
            assert!(
                registry.has_profile(profile_id),
                "profile 缺失: {}",
                profile_id
            );
        }
    }

    /// 验证未知 profile 返回错误
    #[test]
    fn test_unknown_profile_rejected() {
        let registry = AgentProfileRegistry::new_default();
        let result = registry.get_profile("chat.unknown");

        assert!(result.is_err());
        let error = result.err().expect("error should exist");
        assert!(matches!(error, OrchestrationError::ProfileNotFound(_)));
    }

    /// 验证 profile 具备风险等级与输出协议元数据
    #[test]
    fn test_profile_risk_and_output_protocol_available() {
        let registry = AgentProfileRegistry::new_default();
        let profile = registry
            .get_profile("chat.grading")
            .expect("chat.grading should exist");

        assert_eq!(profile.max_tool_risk, ToolRiskLevel::Medium);
        assert_eq!(profile.output_protocol, OutputProtocol::Json);
        assert!(profile.prefer_multimodal);
    }

    /// 验证注册表可从来源抽象构建
    #[test]
    fn test_registry_can_build_from_source() {
        let source = StaticAgentProfileSource::builtin();
        let registry = AgentProfileRegistry::from_source(&source);

        assert!(registry.has_profile("chat.homeroom"));
        assert!(registry.has_profile("search.agentic"));
    }

    /// 验证运行时 profile 快照包含 Chunk3 所需字段
    #[test]
    fn test_runtime_profile_snapshot_contains_tool_fields() {
        let registry = AgentProfileRegistry::new_default();
        let runtime_profile = AgentProfileResolver::get_profile(&registry, "chat.ops")
            .expect("chat.ops should exist");

        assert_eq!(runtime_profile.output_protocol, OutputProtocol::Markdown);
        assert_eq!(runtime_profile.max_tool_risk, ToolRiskLevel::Medium);
        assert!(runtime_profile
            .tool_denylist
            .iter()
            .any(|item| item == "filesystem.delete"));
        assert!(runtime_profile.requires_agentic_search);
    }
}

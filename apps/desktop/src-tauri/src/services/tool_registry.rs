//! 统一工具注册中心（WP-AI-006）
//!
//! 集中管理所有可用工具：内置技能、Python技能、MCP工具。
//! 提供统一的工具发现、查找和权限控制能力。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::error::AppError;
use crate::services::unified_tool::{ToolRiskLevel, UnifiedTool};

/// 工具元数据
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub risk_level: ToolRiskLevel,
    pub source: ToolSource,
}

/// 工具类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    Builtin,
    Skill,
    Mcp,
}

/// 工具来源
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolSource {
    Builtin,
    Skill { skill_id: String },
    Mcp { server_id: String },
}

/// 统一工具注册中心
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn UnifiedTool>>>,
    metadata: RwLock<HashMap<String, ToolMetadata>>,
}

impl ToolRegistry {
    /// 创建新的注册中心
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
        }
    }

    /// 注册工具
    pub fn register(
        &self,
        tool: Arc<dyn UnifiedTool>,
        category: ToolCategory,
        source: ToolSource,
    ) -> Result<(), AppError> {
        let name = tool.name().to_string();
        let metadata = ToolMetadata {
            name: name.clone(),
            description: tool.description().to_string(),
            category,
            risk_level: tool.risk_level(),
            source,
        };

        {
            let mut tools = self
                .tools
                .write()
                .map_err(|_| AppError::Internal(String::from("工具注册中心锁定失败")))?;
            tools.insert(name.clone(), tool);
        }

        {
            let mut meta = self
                .metadata
                .write()
                .map_err(|_| AppError::Internal(String::from("工具元数据锁定失败")))?;
            meta.insert(name, metadata);
        }

        Ok(())
    }

    /// 按名称查找工具
    pub fn get(&self, name: &str) -> Option<Arc<dyn UnifiedTool>> {
        let tools = self.tools.read().ok()?;
        tools.get(name).cloned()
    }

    /// 获取工具元数据
    pub fn get_metadata(&self, name: &str) -> Option<ToolMetadata> {
        let meta = self.metadata.read().ok()?;
        meta.get(name).cloned()
    }

    /// 列出所有工具
    pub fn list_all(&self) -> Vec<ToolMetadata> {
        let meta = self.metadata.read();
        match meta {
            Ok(m) => m.values().cloned().collect(),
            Err(_) => vec![],
        }
    }

    /// 按类别过滤
    pub fn list_by_category(&self, category: ToolCategory) -> Vec<ToolMetadata> {
        let meta = self.metadata.read();
        match meta {
            Ok(m) => m
                .values()
                .filter(|md| md.category == category)
                .cloned()
                .collect(),
            Err(_) => vec![],
        }
    }

    /// 按风险等级过滤
    pub fn list_by_risk(&self, risk_level: ToolRiskLevel) -> Vec<ToolMetadata> {
        let meta = self.metadata.read();
        match meta {
            Ok(m) => m
                .values()
                .filter(|md| md.risk_level == risk_level)
                .cloned()
                .collect(),
            Err(_) => vec![],
        }
    }

    /// 获取指定角色的工具白名单
    ///
    /// 根据Agent角色返回允许使用的工具列表。
    /// 班主任Agent可以访问更多工具，其他角色受限。
    pub fn get_role_tool_allowlist(&self, agent_role: &str) -> Vec<String> {
        let all_tools = self.list_all();

        match agent_role {
            "homeroom" | "admin" => {
                // 班主任和管理员可以使用所有工具
                all_tools.into_iter().map(|md| md.name).collect()
            }
            "subject" => {
                // 学科教师限制部分高危工具
                all_tools
                    .into_iter()
                    .filter(|md| md.risk_level != ToolRiskLevel::High)
                    .map(|md| md.name)
                    .collect()
            }
            _ => {
                // 其他角色只允许低风险工具
                all_tools
                    .into_iter()
                    .filter(|md| md.risk_level == ToolRiskLevel::Low)
                    .map(|md| md.name)
                    .collect()
            }
        }
    }

    /// 注销工具
    pub fn unregister(&self, name: &str) -> Result<(), AppError> {
        {
            let mut tools = self
                .tools
                .write()
                .map_err(|_| AppError::Internal(String::from("工具注册中心锁定失败")))?;
            tools.remove(name);
        }

        {
            let mut meta = self
                .metadata
                .write()
                .map_err(|_| AppError::Internal(String::from("工具元数据锁定失败")))?;
            meta.remove(name);
        }

        Ok(())
    }

    /// 清空所有工具
    pub fn clear(&self) -> Result<(), AppError> {
        {
            let mut tools = self
                .tools
                .write()
                .map_err(|_| AppError::Internal(String::from("工具注册中心锁定失败")))?;
            tools.clear();
        }

        {
            let mut meta = self
                .metadata
                .write()
                .map_err(|_| AppError::Internal(String::from("工具元数据锁定失败")))?;
            meta.clear();
        }

        Ok(())
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局工具注册中心实例
use std::sync::OnceLock;

static GLOBAL_REGISTRY: OnceLock<ToolRegistry> = OnceLock::new();

/// 获取全局工具注册中心
pub fn get_registry() -> &'static ToolRegistry {
    GLOBAL_REGISTRY.get_or_init(ToolRegistry::new)
}

/// 初始化注册中心（应用启动时调用）
pub fn init_registry() -> &'static ToolRegistry {
    get_registry()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::builtin_skills::math_compute::MathComputeSkill;

    #[test]
    fn test_register_and_lookup() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(MathComputeSkill);

        registry
            .register(tool.clone(), ToolCategory::Builtin, ToolSource::Builtin)
            .unwrap();

        let found = registry.get("math.compute");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "math.compute");
    }

    #[test]
    fn test_list_by_category() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(MathComputeSkill);

        registry
            .register(tool.clone(), ToolCategory::Builtin, ToolSource::Builtin)
            .unwrap();

        let builtin_tools = registry.list_by_category(ToolCategory::Builtin);
        assert_eq!(builtin_tools.len(), 1);
        assert_eq!(builtin_tools[0].name, "math.compute");
    }
}

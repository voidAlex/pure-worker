//! 技能工具适配器模块
//!
//! 将 UnifiedTool 协议桥接到 rig-core 的 ToolDyn trait，
//! 使技能可以作为 Agent 工具被 LLM 调用。
//! 内置技能直接通过 UnifiedTool trait 对象调用，无需 DB 查找。

use rig::completion::ToolDefinition;
use rig::tool::{ToolDyn, ToolSet};
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::error::AppError;
use crate::services::builtin_skills;
use crate::services::skill::SkillService;
use crate::services::skill_executor::SkillExecutorService;
use crate::services::unified_tool::UnifiedTool;

/// 内置技能适配器：直接桥接 UnifiedTool trait 对象到 rig ToolDyn。
///
/// 不经过 DB 查找和 SkillExecutorService，直接调用内置技能实现。
pub struct BuiltinToolAdapter {
    tool: Arc<Box<dyn UnifiedTool>>,
}

impl BuiltinToolAdapter {
    pub fn new(tool: Box<dyn UnifiedTool>) -> Self {
        Self {
            tool: Arc::new(tool),
        }
    }
}

impl ToolDyn for BuiltinToolAdapter {
    fn name(&self) -> String {
        self.tool.name().to_string()
    }

    fn definition<'a>(
        &'a self,
        _prompt: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolDefinition> + Send + 'a>> {
        let def = ToolDefinition {
            name: self.tool.name().to_string(),
            description: self.tool.description().to_string(),
            parameters: self.tool.input_schema(),
        };
        Box::pin(async move { def })
    }

    fn call<'a>(
        &'a self,
        args: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, rig::tool::ToolError>> + Send + 'a>,
    > {
        let tool = self.tool.clone();

        Box::pin(async move {
            let input: Value =
                serde_json::from_str(&args).map_err(rig::tool::ToolError::JsonError)?;

            let invoke_id = uuid::Uuid::new_v4().to_string();

            let result = tool.invoke(input, &invoke_id).await.map_err(|e| {
                rig::tool::ToolError::ToolCallError(Box::new(ToolAdapterError(e.to_string())))
            })?;

            serde_json::to_string(&result).map_err(rig::tool::ToolError::JsonError)
        })
    }
}

/// 外部技能适配器（Python 等非内置技能）：通过 DB 查找 + SkillExecutorService 执行。
pub struct SkillToolAdapter {
    skill_name: String,
    skill_description: String,
    input_schema: Value,
    pool: SqlitePool,
}

impl SkillToolAdapter {
    pub fn new(
        skill_name: String,
        skill_description: String,
        input_schema: Value,
        pool: SqlitePool,
    ) -> Self {
        Self {
            skill_name,
            skill_description,
            input_schema,
            pool,
        }
    }

    /// 从数据库中的技能记录构建适配器。
    pub async fn from_skill_name(pool: &SqlitePool, skill_name: &str) -> Result<Self, AppError> {
        let skill = SkillService::get_skill_by_name(pool, skill_name).await?;

        let description = skill
            .description
            .unwrap_or_else(|| format!("技能：{skill_name}"));

        let input_schema = skill
            .config_json
            .as_deref()
            .and_then(|json_str| serde_json::from_str::<Value>(json_str).ok())
            .and_then(|config| config.get("inputSchema").cloned())
            .unwrap_or_else(|| {
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                })
            });

        Ok(Self {
            skill_name: skill.name,
            skill_description: description,
            input_schema,
            pool: pool.clone(),
        })
    }
}

impl ToolDyn for SkillToolAdapter {
    fn name(&self) -> String {
        self.skill_name.clone()
    }

    fn definition<'a>(
        &'a self,
        _prompt: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolDefinition> + Send + 'a>> {
        let def = ToolDefinition {
            name: self.skill_name.clone(),
            description: self.skill_description.clone(),
            parameters: self.input_schema.clone(),
        };
        Box::pin(async move { def })
    }

    fn call<'a>(
        &'a self,
        args: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, rig::tool::ToolError>> + Send + 'a>,
    > {
        let pool = self.pool.clone();
        let skill_name = self.skill_name.clone();

        Box::pin(async move {
            let input: Value =
                serde_json::from_str(&args).map_err(rig::tool::ToolError::JsonError)?;

            let result = SkillExecutorService::execute_skill(&pool, &skill_name, input)
                .await
                .map_err(|e| {
                    rig::tool::ToolError::ToolCallError(Box::new(ToolAdapterError(e.to_string())))
                })?;

            serde_json::to_string(&result).map_err(rig::tool::ToolError::JsonError)
        })
    }
}

/// 构建包含多个技能的 ToolSet，供 Agent 使用。
pub async fn build_skill_toolset(
    pool: &SqlitePool,
    skill_names: &[&str],
) -> Result<ToolSet, AppError> {
    let mut toolset = ToolSet::default();

    for &name in skill_names {
        if let Some(builtin_tool) = builtin_skills::get_builtin_tool(name) {
            toolset.add_tool(BuiltinToolAdapter::new(builtin_tool));
        } else {
            let adapter = SkillToolAdapter::from_skill_name(pool, name).await?;
            toolset.add_tool(adapter);
        }
    }

    Ok(toolset)
}

/// 构建所有已启用技能的 ToolSet。
pub async fn build_all_enabled_skill_toolset(pool: &SqlitePool) -> Result<ToolSet, AppError> {
    let skills = SkillService::list_skills(pool).await?;
    let mut toolset = ToolSet::default();

    for skill in skills {
        if skill.status.as_deref() != Some("enabled") {
            continue;
        }

        if let Some(builtin_tool) = builtin_skills::get_builtin_tool(&skill.name) {
            toolset.add_tool(BuiltinToolAdapter::new(builtin_tool));
            continue;
        }

        let description = skill
            .description
            .unwrap_or_else(|| format!("技能：{}", skill.name));

        let input_schema = skill
            .config_json
            .as_deref()
            .and_then(|json_str| serde_json::from_str::<Value>(json_str).ok())
            .and_then(|config| config.get("inputSchema").cloned())
            .unwrap_or_else(|| {
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                })
            });

        let adapter = SkillToolAdapter::new(skill.name, description, input_schema, pool.clone());
        toolset.add_tool(adapter);
    }

    Ok(toolset)
}

/// 适配器内部错误类型。
#[derive(Debug)]
struct ToolAdapterError(String);

impl std::fmt::Display for ToolAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolAdapterError {}

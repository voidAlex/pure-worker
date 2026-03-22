//! 提示词模板注册表服务模块
//!
//! 提供多模态提示词模板的发现、注册和基于模型能力的选择功能。
//! 支持基于模型能力的模板选择和降级机制。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::AppError;

pub const ACTIVE_RUNTIME_TEMPLATE_NAMES: [&str; 6] = [
    "activity_announcement",
    "chat_homeroom_multimodal",
    "chat_homeroom_text",
    "grading_multimodal_json",
    "parent_communication",
    "semester_comment",
];

pub fn runtime_template_name(profile_id: &str, use_multimodal: bool) -> Option<&'static str> {
    match (profile_id, use_multimodal) {
        ("chat.homeroom", true) => Some("chat_homeroom_multimodal"),
        ("chat.homeroom", false) => Some("chat_homeroom_text"),
        ("chat.grading", _) => Some("grading_multimodal_json"),
        ("chat.communication", _) => Some("parent_communication"),
        ("generation.parent_communication", _) => Some("parent_communication"),
        ("generation.semester_comment", _) => Some("semester_comment"),
        ("generation.activity_announcement", _) => Some("activity_announcement"),
        _ => None,
    }
}

/// 任务类型枚举
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Chat,
    Communication,
    Grading,
    AgenticSearchSummary,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Chat => "chat",
            TaskType::Communication => "communication",
            TaskType::Grading => "grading",
            TaskType::AgenticSearchSummary => "agentic_search_summary",
        }
    }
}

/// 模态类型枚举
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Multimodal,
}

impl Modality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Modality::Text => "text",
            Modality::Multimodal => "multimodal",
        }
    }
}

/// 输出协议枚举
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputProtocol {
    Markdown,
    StructuredJson,
    DraftCard,
}

impl OutputProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputProtocol::Markdown => "markdown",
            OutputProtocol::StructuredJson => "structured_json",
            OutputProtocol::DraftCard => "draft_card",
        }
    }
}

/// 模型能力枚举
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    JsonMode,
    ToolCalling,
    Vision,
    Streaming,
    FunctionCalling,
}

impl ModelCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelCapability::JsonMode => "json_mode",
            ModelCapability::ToolCalling => "tool_calling",
            ModelCapability::Vision => "vision",
            ModelCapability::Streaming => "streaming",
            ModelCapability::FunctionCalling => "function_calling",
        }
    }
}

/// 扩展的模板元数据，支持多模态和能力需求
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtendedTemplateMeta {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub required_variables: Vec<String>,
    pub optional_variables: Option<Vec<String>>,
    pub task_type: TaskType,
    pub modality: Option<Modality>,
    pub capability_requirements: Option<Vec<ModelCapability>>,
    pub output_protocol: Option<OutputProtocol>,
    pub fallback_template: Option<String>,
}

/// 多模态内容项（支持文本或图片）
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentItem {
    Text { content: String },
    Image { url: String, detail: Option<String> },
}

/// 多模态模板内容
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultimodalTemplateContent {
    pub system: Vec<ContentItem>,
    pub user: Vec<ContentItem>,
}

/// 完整的多模态模板
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultimodalPromptTemplate {
    pub meta: ExtendedTemplateMeta,
    pub template: MultimodalTemplateContent,
}

/// 模板数据库记录
#[derive(Debug, Clone, FromRow)]
pub struct PromptTemplateRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub task_type: String,
    pub modality: String,
    pub capability_requirements_json: Option<String>,
    pub output_protocol: String,
    pub fallback_template_id: Option<String>,
    pub is_enabled: i32,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 模型能力集合（用于模板选择）
#[derive(Debug, Clone, Default)]
pub struct ModelCapabilities {
    capabilities: HashSet<ModelCapability>,
}

impl ModelCapabilities {
    pub fn new(capabilities: Vec<ModelCapability>) -> Self {
        Self {
            capabilities: capabilities.into_iter().collect(),
        }
    }

    pub fn supports(&self, capability: &ModelCapability) -> bool {
        self.capabilities.contains(capability)
    }

    pub fn supports_all(&self, required: &[ModelCapability]) -> bool {
        required.iter().all(|c| self.supports(c))
    }
}

/// 模板选择器
///
/// 根据模型能力和任务需求选择最佳模板
pub struct TemplateSelector;

impl TemplateSelector {
    /// 选择最佳模板
    ///
    /// 选择逻辑：
    /// 1. 优先选择完全匹配模型能力的模板
    /// 2. 如果没有完全匹配的，尝试使用支持fallback的模板
    /// 3. 返回找到的模板ID
    pub async fn select_template(
        pool: &SqlitePool,
        task_type: &TaskType,
        model_capabilities: &ModelCapabilities,
        prefer_multimodal: bool,
    ) -> Result<Option<String>, AppError> {
        let task_type_str = task_type.as_str();

        let records = sqlx::query_as::<_, PromptTemplateRecord>(
            "SELECT id, name, version, description, task_type, modality, 
                    capability_requirements_json, output_protocol, fallback_template_id,
                    is_enabled, is_deleted, created_at, updated_at
             FROM prompt_template_registry 
             WHERE task_type = ? AND is_enabled = 1 AND is_deleted = 0
             ORDER BY modality DESC, created_at DESC",
        )
        .bind(task_type_str)
        .fetch_all(pool)
        .await?;

        for record in records {
            let requirements = Self::parse_capabilities(&record.capability_requirements_json);

            if model_capabilities.supports_all(&requirements) {
                if prefer_multimodal && record.modality == "multimodal" {
                    return Ok(Some(record.id));
                }
                if !prefer_multimodal && record.modality == "text" {
                    return Ok(Some(record.id));
                }
                if !prefer_multimodal {
                    return Ok(Some(record.id));
                }
            }
        }

        Ok(None)
    }

    /// 选择带降级机制的模板
    ///
    /// 如果首选模板不支持，自动使用fallback模板
    pub async fn select_with_fallback(
        pool: &SqlitePool,
        task_type: &TaskType,
        model_capabilities: &ModelCapabilities,
        prefer_multimodal: bool,
    ) -> Result<Option<String>, AppError> {
        let mut visited = HashSet::new();
        let mut current_id =
            Self::select_template(pool, task_type, model_capabilities, prefer_multimodal).await?;

        while let Some(id) = current_id {
            if visited.contains(&id) {
                return Err(AppError::Config(String::from("检测到循环fallback依赖")));
            }
            visited.insert(id.clone());

            let record = sqlx::query_as::<_, PromptTemplateRecord>(
                "SELECT id, name, version, description, task_type, modality, 
                        capability_requirements_json, output_protocol, fallback_template_id,
                        is_enabled, is_deleted, created_at, updated_at
                 FROM prompt_template_registry 
                 WHERE id = ? AND is_deleted = 0",
            )
            .bind(&id)
            .fetch_optional(pool)
            .await?;

            match record {
                Some(r) => {
                    let requirements = Self::parse_capabilities(&r.capability_requirements_json);
                    if model_capabilities.supports_all(&requirements) {
                        return Ok(Some(id));
                    }
                    current_id = r.fallback_template_id;
                }
                None => break,
            }
        }

        Ok(None)
    }

    fn parse_capabilities(json_str: &Option<String>) -> Vec<ModelCapability> {
        match json_str {
            Some(json) => serde_json::from_str(json).unwrap_or_default(),
            None => Vec::new(),
        }
    }
}

/// 提示词模板注册表服务
pub struct PromptTemplateRegistry;

impl PromptTemplateRegistry {
    /// 发现和注册所有模板
    ///
    /// 扫描模板目录中的所有TOML文件，并将其注册到数据库
    pub async fn discover_and_register(
        pool: &SqlitePool,
        templates_dir: &Path,
    ) -> Result<Vec<String>, AppError> {
        let mut registered_ids = Vec::new();

        let entries = fs::read_dir(templates_dir).map_err(|error| {
            AppError::FileOperation(format!(
                "读取模板目录失败：{}，错误：{}",
                templates_dir.display(),
                error
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|error| AppError::FileOperation(format!("读取目录条目失败：{error}")))?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }

            match Self::register_from_file(pool, &path).await {
                Ok(id) => registered_ids.push(id),
                Err(error) => {
                    eprintln!("注册模板失败：{}，错误：{}", path.display(), error);
                }
            }
        }

        Ok(registered_ids)
    }

    /// 从单个文件注册模板
    async fn register_from_file(pool: &SqlitePool, path: &Path) -> Result<String, AppError> {
        let content = fs::read_to_string(path).map_err(|error| {
            AppError::FileOperation(format!(
                "读取模板文件失败：{}，错误：{}",
                path.display(),
                error
            ))
        })?;

        let template: MultimodalPromptTemplate = toml::from_str(&content).map_err(|error| {
            AppError::Config(format!(
                "解析模板文件失败：{}，错误：{}",
                path.display(),
                error
            ))
        })?;

        Self::register(pool, template).await
    }

    /// 注册模板到数据库
    pub async fn register(
        pool: &SqlitePool,
        template: MultimodalPromptTemplate,
    ) -> Result<String, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let capability_json = template
            .meta
            .capability_requirements
            .map(|caps| serde_json::to_string(&caps).unwrap_or_default());

        let fallback_id = match template.meta.fallback_template {
            Some(name) => Self::find_template_id_by_name(pool, &name).await?,
            None => None,
        };

        sqlx::query(
            "INSERT INTO prompt_template_registry 
             (id, name, version, description, task_type, modality, 
              capability_requirements_json, output_protocol, fallback_template_id,
              is_enabled, is_deleted, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 0, ?, ?)
             ON CONFLICT(name, version) WHERE is_deleted = 0
             DO UPDATE SET 
                description = excluded.description,
                task_type = excluded.task_type,
                modality = excluded.modality,
                capability_requirements_json = excluded.capability_requirements_json,
                output_protocol = excluded.output_protocol,
                fallback_template_id = excluded.fallback_template_id,
                updated_at = excluded.updated_at",
        )
        .bind(&id)
        .bind(&template.meta.name)
        .bind(&template.meta.version)
        .bind(&template.meta.description)
        .bind(template.meta.task_type.as_str())
        .bind(template.meta.modality.unwrap_or(Modality::Text).as_str())
        .bind(&capability_json)
        .bind(
            template
                .meta
                .output_protocol
                .unwrap_or(OutputProtocol::Markdown)
                .as_str(),
        )
        .bind(&fallback_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(id)
    }

    /// 根据名称查找模板ID
    async fn find_template_id_by_name(
        pool: &SqlitePool,
        name: &str,
    ) -> Result<Option<String>, AppError> {
        let record = sqlx::query_as::<_, PromptTemplateRecord>(
            "SELECT id, name, version, description, task_type, modality, 
                    capability_requirements_json, output_protocol, fallback_template_id,
                    is_enabled, is_deleted, created_at, updated_at
             FROM prompt_template_registry 
             WHERE name = ? AND is_deleted = 0
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;

        Ok(record.map(|r| r.id))
    }

    /// 获取模板记录
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<PromptTemplateRecord, AppError> {
        let record = sqlx::query_as::<_, PromptTemplateRecord>(
            "SELECT id, name, version, description, task_type, modality, 
                    capability_requirements_json, output_protocol, fallback_template_id,
                    is_enabled, is_deleted, created_at, updated_at
             FROM prompt_template_registry 
             WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("模板不存在：{id}")))?;

        Ok(record)
    }

    /// 列出所有模板
    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<PromptTemplateRecord>, AppError> {
        let records = sqlx::query_as::<_, PromptTemplateRecord>(
            "SELECT id, name, version, description, task_type, modality, 
                    capability_requirements_json, output_protocol, fallback_template_id,
                    is_enabled, is_deleted, created_at, updated_at
             FROM prompt_template_registry 
             WHERE is_deleted = 0
             ORDER BY task_type, name",
        )
        .fetch_all(pool)
        .await?;

        Ok(records)
    }

    /// 按任务类型列出模板
    pub async fn list_by_task_type(
        pool: &SqlitePool,
        task_type: &TaskType,
    ) -> Result<Vec<PromptTemplateRecord>, AppError> {
        let records = sqlx::query_as::<_, PromptTemplateRecord>(
            "SELECT id, name, version, description, task_type, modality, 
                    capability_requirements_json, output_protocol, fallback_template_id,
                    is_enabled, is_deleted, created_at, updated_at
             FROM prompt_template_registry 
             WHERE task_type = ? AND is_deleted = 0
             ORDER BY name",
        )
        .bind(task_type.as_str())
        .fetch_all(pool)
        .await?;

        Ok(records)
    }

    /// 禁用模板
    pub async fn disable(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE prompt_template_registry SET is_enabled = 0, updated_at = ? WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 启用模板
    pub async fn enable(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE prompt_template_registry SET is_enabled = 1, updated_at = ? WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 软删除模板
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE prompt_template_registry SET is_deleted = 1, updated_at = ? WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// 从文件系统加载多模态模板（用于渲染）
pub fn load_multimodal_template(path: &Path) -> Result<MultimodalPromptTemplate, AppError> {
    let content = fs::read_to_string(path).map_err(|error| {
        AppError::FileOperation(format!(
            "读取模板文件失败：{}，错误：{}",
            path.display(),
            error
        ))
    })?;

    let template = toml::from_str(&content).map_err(|error| {
        AppError::Config(format!(
            "解析模板文件失败：{}，错误：{}",
            path.display(),
            error
        ))
    })?;

    Ok(template)
}

/// 渲染多模态模板（变量替换）
pub fn render_multimodal_template(
    template: &MultimodalPromptContent,
    variables: &HashMap<String, String>,
) -> Result<MultimodalTemplateContent, AppError> {
    match template {
        MultimodalPromptContent::Multimodal { system, user } => {
            let rendered_system = render_content_items(system, variables)?;
            let rendered_user = render_content_items(user, variables)?;

            Ok(MultimodalTemplateContent {
                system: rendered_system,
                user: rendered_user,
            })
        }
        MultimodalPromptContent::Legacy { system, user } => {
            let rendered_system = render_text(system, variables)?;
            let rendered_user = render_text(user, variables)?;

            Ok(MultimodalTemplateContent {
                system: vec![ContentItem::Text {
                    content: rendered_system,
                }],
                user: vec![ContentItem::Text {
                    content: rendered_user,
                }],
            })
        }
    }
}

pub fn render_content_items(
    items: &[ContentItem],
    variables: &HashMap<String, String>,
) -> Result<Vec<ContentItem>, AppError> {
    items
        .iter()
        .map(|item| match item {
            ContentItem::Text { content } => {
                let rendered = render_text(content, variables)?;
                Ok(ContentItem::Text { content: rendered })
            }
            ContentItem::Image { url, detail } => {
                let rendered_url = render_text(url, variables)?;
                Ok(ContentItem::Image {
                    url: rendered_url,
                    detail: detail.clone(),
                })
            }
        })
        .collect()
}

pub fn render_text(text: &str, variables: &HashMap<String, String>) -> Result<String, AppError> {
    let if_block_regex = Regex::new(r"(?s)\{\{#if\s+([a-zA-Z0-9_]+)\s*\}\}(.*?)\{\{/if\}\}")
        .map_err(|error| AppError::Config(format!("条件块解析规则初始化失败：{error}")))?;

    let with_if_resolved = if_block_regex
        .replace_all(text, |captures: &regex::Captures<'_>| {
            let var_name = captures.get(1).map_or("", |m| m.as_str());
            let block_content = captures.get(2).map_or("", |m| m.as_str());

            if variables
                .get(var_name)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
            {
                block_content.to_string()
            } else {
                String::new()
            }
        })
        .to_string();

    let variable_regex = Regex::new(r"\{\{\s*([a-zA-Z0-9_]+)\s*\}\}")
        .map_err(|error| AppError::Config(format!("变量替换规则初始化失败：{error}")))?;

    let rendered = variable_regex
        .replace_all(&with_if_resolved, |captures: &regex::Captures<'_>| {
            let var_name = captures.get(1).map_or("", |m| m.as_str());
            if let Some(value) = variables.get(var_name) {
                value.clone()
            } else {
                captures
                    .get(0)
                    .map_or_else(String::new, |matched| matched.as_str().to_string())
            }
        })
        .to_string();

    Ok(rendered)
}

/// 用于反序列化的模板内容结构（支持多模态和纯文本兼容）
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MultimodalPromptContent {
    Multimodal {
        system: Vec<ContentItem>,
        user: Vec<ContentItem>,
    },
    Legacy {
        system: String,
        user: String,
    },
}

impl MultimodalPromptContent {
    pub fn to_content_items(self) -> (Vec<ContentItem>, Vec<ContentItem>) {
        match self {
            MultimodalPromptContent::Multimodal { system, user } => (system, user),
            MultimodalPromptContent::Legacy { system, user } => {
                let system_items = vec![ContentItem::Text { content: system }];
                let user_items = vec![ContentItem::Text { content: user }];
                (system_items, user_items)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_capabilities() {
        let caps = ModelCapabilities::new(vec![ModelCapability::JsonMode, ModelCapability::Vision]);

        assert!(caps.supports(&ModelCapability::JsonMode));
        assert!(caps.supports(&ModelCapability::Vision));
        assert!(!caps.supports(&ModelCapability::ToolCalling));

        assert!(caps.supports_all(&[ModelCapability::JsonMode, ModelCapability::Vision]));
        assert!(!caps.supports_all(&[ModelCapability::JsonMode, ModelCapability::ToolCalling]));
    }

    #[test]
    fn test_render_text_with_variables() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "张三".to_string());
        vars.insert("subject".to_string(), "数学".to_string());

        let template = "你好{{name}}，你的{{subject}}成绩很好";
        let result = render_text(template, &vars).unwrap();

        assert_eq!(result, "你好张三，你的数学成绩很好");
    }

    #[test]
    fn test_render_text_with_conditionals() {
        let mut vars = HashMap::new();
        vars.insert("has_bonus".to_string(), "yes".to_string());

        let template = "基础内容{{#if has_bonus}}，有额外奖励{{/if}}，结束";
        let result = render_text(template, &vars).unwrap();

        assert_eq!(result, "基础内容，有额外奖励，结束");
    }

    #[test]
    fn test_render_text_with_empty_conditional() {
        let vars = HashMap::<String, String>::new();

        let template = "基础内容{{#if missing}}，有条件内容{{/if}}，结束";
        let result = render_text(template, &vars).unwrap();

        assert_eq!(result, "基础内容，结束");
    }
}

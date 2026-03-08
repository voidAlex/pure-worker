//! 提示词模板服务模块
//!
//! 提供版本化提示词模板加载、变量校验与渲染能力。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use regex::Regex;
use serde::Deserialize;

use crate::error::AppError;

/// 模板元数据。
#[derive(Debug, Clone, Deserialize)]
pub struct TemplateMeta {
    /// 模板名称。
    pub name: String,
    /// 模板版本。
    pub version: String,
    /// 模板描述。
    pub description: String,
    /// 渲染时必须提供的变量列表。
    pub required_variables: Vec<String>,
}

/// 模板内容。
#[derive(Debug, Clone, Deserialize)]
pub struct TemplateContent {
    /// 系统提示词模板。
    pub system: String,
    /// 用户提示词模板。
    pub user: String,
}

/// 完整模板。
#[derive(Debug, Clone, Deserialize)]
pub struct PromptTemplate {
    /// 模板元数据。
    pub meta: TemplateMeta,
    /// 模板正文。
    pub template: TemplateContent,
}

/// 渲染后的提示词。
#[derive(Debug, Clone)]
pub struct RenderedPrompt {
    /// 渲染后的系统提示词。
    pub system: String,
    /// 渲染后的用户提示词。
    pub user: String,
}

/// 提示词模板服务。
pub struct PromptTemplateService;

impl PromptTemplateService {
    /// 从文件加载模板。
    pub fn load_template(
        templates_dir: &Path,
        template_name: &str,
    ) -> Result<PromptTemplate, AppError> {
        Self::validate_template_name(template_name)?;

        let template_path = templates_dir
            .join("templates")
            .join(format!("{template_name}.toml"));

        let content = fs::read_to_string(&template_path).map_err(|error| {
            AppError::FileOperation(format!(
                "读取提示词模板失败：{}，错误：{}",
                template_path.display(),
                error
            ))
        })?;

        let template = toml::from_str::<PromptTemplate>(&content).map_err(|error| {
            AppError::Config(format!(
                "解析提示词模板失败：{}，错误：{}",
                template_path.display(),
                error
            ))
        })?;

        Ok(template)
    }

    /// 验证变量是否齐全。
    pub fn validate_variables(
        template: &PromptTemplate,
        variables: &HashMap<String, String>,
    ) -> Result<(), AppError> {
        let missing_variables = template
            .meta
            .required_variables
            .iter()
            .filter(|key| {
                variables
                    .get(*key)
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<String>>();

        if !missing_variables.is_empty() {
            return Err(AppError::InvalidInput(format!(
                "提示词模板缺少必填变量：{}",
                missing_variables.join(", ")
            )));
        }

        Ok(())
    }

    /// 渲染模板（变量替换）。
    pub fn render(
        template: &PromptTemplate,
        variables: &HashMap<String, String>,
    ) -> Result<RenderedPrompt, AppError> {
        Self::validate_variables(template, variables)?;

        let rendered_system = Self::render_text(&template.template.system, variables)?;
        let rendered_user = Self::render_text(&template.template.user, variables)?;

        Ok(RenderedPrompt {
            system: rendered_system,
            user: rendered_user,
        })
    }

    /// 校验模板名称是否合法，避免路径穿越。
    fn validate_template_name(template_name: &str) -> Result<(), AppError> {
        if template_name.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from(
                "template_name 不能为空",
            )));
        }

        let name_regex = Regex::new(r"^[a-zA-Z0-9_\-]+$")
            .map_err(|error| AppError::Config(format!("模板名称校验规则初始化失败：{error}")))?;

        if !name_regex.is_match(template_name) {
            return Err(AppError::InvalidInput(String::from(
                "template_name 非法，仅支持字母、数字、下划线和中划线",
            )));
        }

        Ok(())
    }

    /// 渲染文本：先处理条件块，再做变量替换。
    fn render_text(text: &str, variables: &HashMap<String, String>) -> Result<String, AppError> {
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
}

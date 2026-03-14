//! 技能注册数据模型模块
//!
//! 定义技能注册表记录、创建/更新输入以及健康检查结果结构。
//! 遵循 Agent Skills 官方规范 (https://agentskills.io/specification)

use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

/// 技能注册表记录。
/// 扩展以支持 Agent Skills 官方规范的所有字段。
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
    pub permission_scope: Option<String>,
    pub status: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub skill_type: String,
    pub env_path: Option<String>,
    pub config_json: Option<String>,
    pub updated_at: Option<String>,
    pub health_status: String,
    pub last_health_check: Option<String>,
    // Agent Skills 规范新增字段
    /// 许可证名称或引用
    pub license: Option<String>,
    /// 环境兼容性要求
    pub compatibility: Option<String>,
    /// 元数据（JSON 格式存储）
    pub metadata_json: Option<String>,
    /// 允许使用的工具列表（空格分隔）
    pub allowed_tools: Option<String>,
    /// SKILL.md 正文内容（渐进式加载）
    pub body_content: Option<String>,
    /// 入口脚本路径（相对于技能目录）
    pub entry_script: Option<String>,
}

/// 创建技能输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateSkillInput {
    pub name: String,
    pub version: Option<String>,
    pub source: Option<String>,
    pub permission_scope: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub skill_type: String,
    /// Python 技能虚拟环境路径（Python 技能必填，内置技能为空）。
    pub env_path: Option<String>,
    pub config_json: Option<String>,
    // Agent Skills 规范新增字段
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata_json: Option<String>,
    pub allowed_tools: Option<String>,
    pub body_content: Option<String>,
    pub entry_script: Option<String>,
}

/// 更新技能输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdateSkillInput {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub permission_scope: Option<String>,
    pub config_json: Option<String>,
    pub status: Option<String>,
    // Agent Skills 规范新增字段
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata_json: Option<String>,
    pub allowed_tools: Option<String>,
    pub body_content: Option<String>,
    pub entry_script: Option<String>,
}

/// 技能健康检查结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillHealthResult {
    pub name: String,
    pub health_status: String,
    pub message: String,
    pub checked_at: String,
}

/// Agent Skills 规范 frontmatter 结构。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillFrontmatter {
    /// 技能名称（必填）
    pub name: String,
    /// 技能描述（必填）
    pub description: String,
    /// 许可证（可选）
    pub license: Option<String>,
    /// 环境兼容性（可选）
    pub compatibility: Option<String>,
    /// 元数据（可选）
    pub metadata: Option<HashMap<String, String>>,
    /// 允许使用的工具（可选，空格分隔）
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Option<String>,
}

impl SkillFrontmatter {
    /// 验证技能名称是否符合 Agent Skills 规范。
    ///
    /// 规范要求：
    /// - 1-64 个字符
    /// - 只能包含小写字母 a-z、数字 0-9 和连字符 -
    /// - 不能以连字符开头或结尾
    /// - 不能包含连续的连字符 (--)
    /// - 必须与父目录名称一致
    pub fn validate_name(&self, expected_name: Option<&str>) -> Result<(), String> {
        let name = &self.name;

        // 长度校验
        if name.is_empty() || name.len() > 64 {
            return Err(format!(
                "技能名称 '{}' 长度不合法：必须为 1-64 个字符，当前为 {} 个字符",
                name,
                name.len()
            ));
        }

        // 不能以连字符开头或结尾
        if name.starts_with('-') || name.ends_with('-') {
            return Err(format!("技能名称 '{}' 不能以连字符开头或结尾", name));
        }

        // 不能包含连续的连字符
        if name.contains("--") {
            return Err(format!("技能名称 '{}' 不能包含连续的连字符（--）", name));
        }

        // 只能包含小写字母、数字和连字符
        for ch in name.chars() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
                return Err(format!(
                    "技能名称 '{}' 包含非法字符 '{}'：只能使用小写字母 a-z、数字 0-9 和连字符 -",
                    name, ch
                ));
            }
        }

        // 验证与父目录名称一致（如果提供）
        if let Some(expected) = expected_name {
            if name != expected {
                return Err(format!(
                    "技能名称 '{}' 必须与父目录名称 '{}' 一致",
                    name, expected
                ));
            }
        }

        Ok(())
    }

    /// 验证描述是否符合规范。
    pub fn validate_description(&self) -> Result<(), String> {
        let desc = &self.description;

        if desc.is_empty() {
            return Err(String::from("技能描述不能为空"));
        }

        if desc.len() > 1024 {
            return Err(format!(
                "技能描述长度超过限制：最大 1024 个字符，当前为 {} 个字符",
                desc.len()
            ));
        }

        Ok(())
    }

    /// 验证兼容性字段。
    pub fn validate_compatibility(&self) -> Result<(), String> {
        if let Some(ref compat) = self.compatibility {
            if compat.len() > 500 {
                return Err(format!(
                    "兼容性字段长度超过限制：最大 500 个字符，当前为 {} 个字符",
                    compat.len()
                ));
            }
        }

        Ok(())
    }

    /// 执行完整的 Agent Skills 规范验证。
    pub fn validate(&self, expected_name: Option<&str>) -> Result<(), String> {
        self.validate_name(expected_name)?;
        self.validate_description()?;
        self.validate_compatibility()?;
        Ok(())
    }
}

/// 技能内容（渐进式加载）。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillContent {
    /// frontmatter 元数据
    pub frontmatter: SkillFrontmatter,
    /// Markdown 正文内容
    pub body: String,
    /// 技能目录路径
    pub source_path: String,
    /// 可用的脚本文件（scripts/ 目录下）
    pub available_scripts: Vec<String>,
    /// 可用的引用文件（references/ 目录下）
    pub available_references: Vec<String>,
    /// 可用的资源文件（assets/ 目录下）
    pub available_assets: Vec<String>,
}

/// 技能元数据（仅 frontmatter，用于列表展示）。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata_json: Option<String>,
    pub allowed_tools: Option<String>,
    pub source_path: String,
    pub skill_type: String,
    pub already_installed: bool,
}

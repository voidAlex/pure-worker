//! Soul.md / User.md 文件管理服务模块
//!
//! 提供 soul.md（项目级）和 user.md（用户个人）文件的读取、解析、
//! 自动创建和内容同步功能。支持 Markdown frontmatter 解析。

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use regex::Regex;

use crate::error::AppError;
use crate::models::teacher_memory::{SoulMdContent, SoulMdFrontmatter, SoulMdSection};

/// Soul.md 文件默认内容
pub const DEFAULT_SOUL_MD: &str = r#"---
version: "1.0"
description: "PureWorker AI 助手行为指南"
author: "PureWorker"
last_updated: ""
tags: ["behavior", "preferences", "assistant"]
---

# PureWorker AI 助手行为指南

本文档定义 AI 助手在 PureWorker 桌面应用中的行为准则、输出风格和交互偏好。

## 角色定位

- **身份**: 教师的智能助手，专注于教务管理、作业批改和家校沟通
- **语气**: 专业、友善、高效
- **原则**: 本地优先、隐私保护、教师确认后生效

## 输出风格偏好

### 一般输出
- 结构化输出，使用层级标题和项目符号
- 关键信息前置，细节随后
- 适当使用中文标点符号

### 评语生成
- 基于学生实际表现的具体观察
- 正面与建设性反馈相结合
- 避免泛泛而谈的套话

### 家校沟通
- 共情家长视角，用语礼貌得体
- 问题描述清晰，提供可行的建议
- 突出学生的进步和潜力

## 格式偏好

- 日期格式: YYYY-MM-DD
- 列表使用层级缩进和 emoji 图标
- 数值使用阿拉伯数字配合中文量词

## 工作流偏好

- 高风险操作需二次确认
- 外发内容默认启用脱敏检查
- 自动保存间隔: 30 秒
- 长任务提供步骤级进度反馈

## 记忆系统

- 尊重教师的显式偏好设置
- 不主动应用推断的偏好（需确认）
- 定期同步数据库偏好到本文档
"#;

/// User.md 文件默认内容
pub const DEFAULT_USER_MD: &str = r#"---
version: "1.0"
description: "教师个人偏好配置"
author: ""
last_updated: ""
tags: ["personal", "preferences", "customization"]
---

# 教师个人偏好配置

本文档记录教师的个人偏好、习惯用语和定制化需求。

## 个人习惯

- 称呼偏好:
- 常用语:
- 避讳词:

## 教学风格

- 学科特点:
- 班级规模:
- 关注重点:

## 沟通偏好

### 与家长沟通
- 语气风格:
- 信息详略:
- 回复时效期望:

### 与学生交流
- 激励方式:
- 批评措辞:
- 鼓励用语:

## 自定义快捷用语

- 表扬模板:
- 提醒模板:
- 通知模板:

## 特殊需求

- 格式定制:
- 输出长度偏好:
- 其他个性化需求:
"#;

/// Soul.md 管理服务
pub struct SoulMdManager;

impl SoulMdManager {
    /// 获取 workspace 目录下的 soul.md 路径
    pub fn get_soul_md_path(workspace_path: &Path) -> PathBuf {
        workspace_path.join("soul.md")
    }

    /// 获取 workspace 目录下的 user.md 路径
    pub fn get_user_md_path(workspace_path: &Path) -> PathBuf {
        workspace_path.join("user.md")
    }

    /// 确保 soul.md 文件存在（不存在则创建默认内容）
    pub fn ensure_soul_md(workspace_path: &Path) -> Result<PathBuf, AppError> {
        let soul_path = Self::get_soul_md_path(workspace_path);

        if !soul_path.exists() {
            fs::create_dir_all(workspace_path)
                .map_err(|e| AppError::FileOperation(format!("创建工作目录失败: {}", e)))?;

            let content = DEFAULT_SOUL_MD.replace(
                "last_updated: \"\"",
                &format!("last_updated: \"{}\"", Utc::now().to_rfc3339()),
            );

            fs::write(&soul_path, content)
                .map_err(|e| AppError::FileOperation(format!("创建 soul.md 失败: {}", e)))?;
        }

        Ok(soul_path)
    }

    /// 确保 user.md 文件存在（不存在则创建默认内容）
    pub fn ensure_user_md(workspace_path: &Path) -> Result<PathBuf, AppError> {
        let user_path = Self::get_user_md_path(workspace_path);

        if !user_path.exists() {
            fs::create_dir_all(workspace_path)
                .map_err(|e| AppError::FileOperation(format!("创建工作目录失败: {}", e)))?;

            let content = DEFAULT_USER_MD.replace(
                "last_updated: \"\"",
                &format!("last_updated: \"{}\"", Utc::now().to_rfc3339()),
            );

            fs::write(&user_path, content)
                .map_err(|e| AppError::FileOperation(format!("创建 user.md 失败: {}", e)))?;
        }

        Ok(user_path)
    }

    /// 读取并解析 soul.md 文件
    pub fn load_soul_md(workspace_path: &Path) -> Result<SoulMdContent, AppError> {
        let soul_path = Self::ensure_soul_md(workspace_path)?;
        Self::parse_markdown_file(&soul_path)
    }

    /// 读取并解析 user.md 文件
    pub fn load_user_md(workspace_path: &Path) -> Result<SoulMdContent, AppError> {
        let user_path = Self::ensure_user_md(workspace_path)?;
        Self::parse_markdown_file(&user_path)
    }

    /// 重新加载 soul.md（强制重新创建）
    pub fn reload_soul_md(
        workspace_path: &Path,
        force_create: bool,
    ) -> Result<SoulMdContent, AppError> {
        let soul_path = Self::get_soul_md_path(workspace_path);

        if force_create && soul_path.exists() {
            fs::remove_file(&soul_path)
                .map_err(|e| AppError::FileOperation(format!("删除旧 soul.md 失败: {}", e)))?;
        }

        Self::load_soul_md(workspace_path)
    }

    /// 解析 Markdown 文件，提取 frontmatter 和章节
    fn parse_markdown_file(file_path: &Path) -> Result<SoulMdContent, AppError> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| AppError::FileOperation(format!("读取文件失败: {}", e)))?;

        let (frontmatter, body) = Self::extract_frontmatter(&content)?;
        let sections = Self::extract_sections(&body);

        Ok(SoulMdContent {
            version: frontmatter.version,
            description: frontmatter.description,
            sections,
            raw_content: content,
        })
    }

    /// 提取 YAML frontmatter
    fn extract_frontmatter(content: &str) -> Result<(SoulMdFrontmatter, String), AppError> {
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() || lines[0].trim() != "---" {
            return Ok((
                SoulMdFrontmatter {
                    version: None,
                    description: None,
                    author: None,
                    last_updated: None,
                    tags: None,
                },
                content.to_string(),
            ));
        }

        let mut frontmatter_lines = Vec::new();
        let mut end_index = 0;

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                end_index = i;
                break;
            }
            frontmatter_lines.push(*line);
        }

        if end_index == 0 {
            return Ok((
                SoulMdFrontmatter {
                    version: None,
                    description: None,
                    author: None,
                    last_updated: None,
                    tags: None,
                },
                content.to_string(),
            ));
        }

        let frontmatter_yaml = frontmatter_lines.join("\n");
        let frontmatter: SoulMdFrontmatter = serde_yaml::from_str(&frontmatter_yaml)
            .map_err(|e| AppError::InvalidInput(format!("解析 frontmatter 失败: {}", e)))?;

        let body = lines[end_index + 1..].join("\n");

        Ok((frontmatter, body))
    }

    /// 提取 Markdown 章节
    fn extract_sections(content: &str) -> Vec<SoulMdSection> {
        let mut sections = Vec::new();
        let mut current_title = String::from("前言");
        let mut current_content = Vec::new();
        let mut current_level = 1;

        let header_regex = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();

        for line in content.lines() {
            if let Some(captures) = header_regex.captures(line) {
                // 保存上一个章节
                if !current_content.is_empty() {
                    sections.push(SoulMdSection {
                        title: current_title.clone(),
                        content: current_content.join("\n").trim().to_string(),
                        level: current_level,
                    });
                }

                // 开始新章节
                let hashes = captures.get(1).unwrap().as_str();
                current_level = hashes.len() as i32;
                current_title = captures.get(2).unwrap().as_str().to_string();
                current_content.clear();
            } else {
                current_content.push(line);
            }
        }

        // 保存最后一个章节
        if !current_content.is_empty() || !sections.is_empty() {
            sections.push(SoulMdSection {
                title: current_title,
                content: current_content.join("\n").trim().to_string(),
                level: current_level,
            });
        }

        sections
    }

    /// 更新 soul.md 的内容（保留 frontmatter）
    pub fn update_soul_md_content(
        workspace_path: &Path,
        new_content: &str,
    ) -> Result<(), AppError> {
        let soul_path = Self::get_soul_md_path(workspace_path);

        let existing_content = if soul_path.exists() {
            fs::read_to_string(&soul_path)
                .map_err(|e| AppError::FileOperation(format!("读取 soul.md 失败: {}", e)))?
        } else {
            DEFAULT_SOUL_MD.to_string()
        };

        let (frontmatter, _) = Self::extract_frontmatter(&existing_content)?;

        // 构建新的 frontmatter YAML
        let frontmatter_yaml = serde_yaml::to_string(&frontmatter)
            .map_err(|e| AppError::Internal(format!("序列化 frontmatter 失败: {}", e)))?;

        let updated_content = format!(
            "---\n{}---\n\n{}",
            frontmatter_yaml,
            new_content.trim_start()
        );

        fs::write(&soul_path, updated_content)
            .map_err(|e| AppError::FileOperation(format!("写入 soul.md 失败: {}", e)))?;

        Ok(())
    }

    /// 更新 user.md 的内容
    pub fn update_user_md_content(
        workspace_path: &Path,
        new_content: &str,
    ) -> Result<(), AppError> {
        let user_path = Self::get_user_md_path(workspace_path);

        let existing_content = if user_path.exists() {
            fs::read_to_string(&user_path)
                .map_err(|e| AppError::FileOperation(format!("读取 user.md 失败: {}", e)))?
        } else {
            DEFAULT_USER_MD.to_string()
        };

        let (frontmatter, _) = Self::extract_frontmatter(&existing_content)?;

        let frontmatter_yaml = serde_yaml::to_string(&frontmatter)
            .map_err(|e| AppError::Internal(format!("序列化 frontmatter 失败: {}", e)))?;

        let updated_content = format!(
            "---\n{}---\n\n{}",
            frontmatter_yaml,
            new_content.trim_start()
        );

        fs::write(&user_path, updated_content)
            .map_err(|e| AppError::FileOperation(format!("写入 user.md 失败: {}", e)))?;

        Ok(())
    }

    /// 将数据库偏好同步到 soul.md 文件
    pub fn sync_preferences_to_soul_md(
        workspace_path: &Path,
        preferences: &[(String, String)],
    ) -> Result<(), AppError> {
        let mut sections = Vec::new();

        sections.push("## 活跃偏好设置\n\n".to_string());

        for (key, value) in preferences {
            sections.push(format!("- **{}**: {}", key, value));
        }

        let content = sections.join("\n");

        // 读取现有内容
        let soul_path = Self::get_soul_md_path(workspace_path);
        let existing = if soul_path.exists() {
            fs::read_to_string(&soul_path)
                .map_err(|e| AppError::FileOperation(format!("读取 soul.md 失败: {}", e)))?
        } else {
            DEFAULT_SOUL_MD.to_string()
        };

        // 检查是否已有活跃偏好设置章节
        let preferences_section = format!("\n{}\n", content);

        let updated = if existing.contains("## 活跃偏好设置") {
            // 替换现有章节（使用支持的模式）
            let section_regex = Regex::new(r"\n## [^\n]+")
                .map_err(|e| AppError::Internal(format!("正则编译失败: {}", e)))?;

            // 找到 "## 活跃偏好设置" 的位置
            if let Some(start_pos) = existing.find("## 活跃偏好设置") {
                let before = &existing[..start_pos];
                let after_section = section_regex.find_iter(&existing[start_pos..]).nth(1);
                let after = after_section
                    .map(|m| {
                        let end_pos = start_pos + m.start();
                        &existing[end_pos..]
                    })
                    .unwrap_or("");
                format!("{}{}\n{}", before, preferences_section.trim(), after)
            } else {
                // 理论上不会发生，因为前面检查了 contains
                format!("{}\n\n{}", existing.trim(), preferences_section.trim())
            }
        } else {
            // 追加到文件末尾
            format!("{}\n\n{}", existing.trim(), preferences_section.trim())
        };

        fs::write(&soul_path, updated)
            .map_err(|e| AppError::FileOperation(format!("写入 soul.md 失败: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let content = r#"---
version: "1.0"
description: "Test"
---

# Body content

Some text."#;

        let (frontmatter, body) = SoulMdManager::extract_frontmatter(content).unwrap();
        assert_eq!(frontmatter.version, Some("1.0".to_string()));
        assert_eq!(frontmatter.description, Some("Test".to_string()));
        assert!(body.contains("# Body content"));
    }

    #[test]
    fn test_extract_sections() {
        let content = r#"# Section 1
Content 1

## Section 2
Content 2"#;

        let sections = SoulMdManager::extract_sections(content);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "Section 1");
        assert_eq!(sections[1].title, "Section 2");
    }
}

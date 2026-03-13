//! 技能自动发现模块
//!
//! 扫描约定目录（`.agents/skills/`）自动发现并注册第三方技能。
//! 支持项目级和用户级两个扫描路径，项目级技能覆盖用户级同名技能。
//!
//! 技能目录规范：每个技能子目录必须包含 `SKILL.md`，
//! 其 YAML frontmatter 中的 `name` 和 `description` 为必填字段。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::models::skill::{CreateSkillInput, UpdateSkillInput};
use crate::services::audit::AuditService;
use crate::services::path_whitelist::PathWhitelistService;
use crate::services::skill::SkillService;

/// 发现的技能描述结构。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DiscoveredSkill {
    /// 技能名称（来自 SKILL.md frontmatter）。
    pub name: String,
    /// 技能描述（来自 SKILL.md frontmatter）。
    pub description: String,
    /// 技能版本（可选，来自 SKILL.md frontmatter）。
    pub version: Option<String>,
    /// 技能类型（固定为 "python"）。
    pub skill_type: String,
    /// 技能目录的绝对路径。
    pub source_path: String,
    /// 是否已安装到数据库。
    pub already_installed: bool,
}

/// 技能自动发现服务。
pub struct SkillDiscoveryService;

impl SkillDiscoveryService {
    /// 扫描工作区和用户目录，发现可用技能。
    ///
    /// 扫描路径优先级：项目级 > 用户级（同名技能以项目级为准）。
    /// - 项目级：`{workspace_path}/.agents/skills/`
    /// - 用户级：`~/.agents/skills/`（通过 HOME 或 USERPROFILE 环境变量定位）
    pub async fn discover_skills(
        pool: &SqlitePool,
        workspace_path: &Path,
    ) -> Result<Vec<DiscoveredSkill>, AppError> {
        let mut skills_map: HashMap<String, DiscoveredSkill> = HashMap::new();

        // 获取已注册技能列表，用于判断是否已安装
        let existing_skills = SkillService::list_skills(pool).await?;
        let existing_names: Vec<String> = existing_skills.iter().map(|s| s.name.clone()).collect();

        // 扫描用户级目录（优先级低，先扫描）
        let user_skills_dir = Self::get_user_skills_dir();
        if let Some(dir) = &user_skills_dir {
            Self::scan_directory(dir, &existing_names, &mut skills_map)?;
        }

        // 扫描项目级目录（优先级高，覆盖同名）
        // 校验 .agents/skills 安全性（symlink 防护 + workspace 边界校验）
        match PathWhitelistService::validate_skills_dir(workspace_path) {
            Ok((_canonical_workspace, project_skills_dir)) => {
                if project_skills_dir.exists() {
                    // 扫描前二次校验（防止 TOCTOU：validate 与 scan 之间 .agents 被替换为 symlink）
                    match PathWhitelistService::validate_skills_dir(workspace_path) {
                        Ok((_cw2, verified_skills_dir)) => {
                            Self::scan_directory(
                                &verified_skills_dir,
                                &existing_names,
                                &mut skills_map,
                            )?;
                        }
                        Err(e2) => {
                            eprintln!("[技能发现] 扫描前二次校验失败，跳过扫描：{e2}");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[技能发现] 项目级技能目录校验失败，跳过扫描：{e}");
            }
        }

        let mut result: Vec<DiscoveredSkill> = skills_map.into_values().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    /// 发现并自动注册新技能。
    ///
    /// 扫描后对未安装的技能自动创建注册记录，返回所有发现的技能列表。
    pub async fn discover_and_register_new(
        pool: &SqlitePool,
        workspace_path: &Path,
    ) -> Result<Vec<DiscoveredSkill>, AppError> {
        let mut discovered = Self::discover_skills(pool, workspace_path).await?;

        for skill in &mut discovered {
            if skill.already_installed {
                continue;
            }

            let input = CreateSkillInput {
                name: skill.name.clone(),
                version: skill.version.clone(),
                source: Some(skill.source_path.clone()),
                permission_scope: Some(String::from("read_only")),
                display_name: Some(skill.name.clone()),
                description: Some(skill.description.clone()),
                skill_type: skill.skill_type.clone(),
                env_path: None,
                config_json: None,
            };

            let created = SkillService::create_skill(pool, input).await?;
            skill.already_installed = true;

            // Python 技能在自动发现时没有环境（env_path 为空），
            // 标记为 disabled + unhealthy，需用户手动安装环境后启用。
            // 如果标记失败则回滚（删除刚创建的记录），避免残留 enabled + 无 env_path 的不可用技能。
            if skill.skill_type == "python" {
                let update = UpdateSkillInput {
                    display_name: None,
                    description: None,
                    permission_scope: None,
                    config_json: None,
                    status: Some(String::from("disabled")),
                };
                if let Err(e) = SkillService::update_skill(pool, &created.id, update).await {
                    eprintln!(
                        "[技能发现] 标记 Python 技能 '{}' 为 disabled 失败，回滚注册：{e}",
                        skill.name
                    );
                    let _ = SkillService::delete_skill(pool, &created.id).await;
                    skill.already_installed = false;
                    continue;
                }

                let now = chrono::Utc::now().to_rfc3339();
                if let Err(e) = sqlx::query(
                    "UPDATE skill_registry SET health_status = 'unhealthy', last_health_check = ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
                )
                .bind(&now)
                .bind(&now)
                .bind(&created.id)
                .execute(pool)
                .await
                {
                    eprintln!(
                        "[技能发现] 标记 Python 技能 '{}' 健康状态为 unhealthy 失败，回滚注册：{e}",
                        skill.name
                    );
                    let _ = SkillService::delete_skill(pool, &created.id).await;
                    skill.already_installed = false;
                    continue;
                }
            }

            if let Err(e) = AuditService::log(
                pool,
                "system",
                "auto_register_skill",
                "skill_registry",
                Some(&created.id),
                "medium",
                false,
            )
            .await
            {
                eprintln!("[审计日志] 记录技能自动注册审计失败：{e}");
            }
        }

        Ok(discovered)
    }

    /// 扫描指定目录下的技能子目录。
    ///
    /// 遍历目录中的子文件夹，查找包含 `SKILL.md` 的目录并解析 frontmatter。
    fn scan_directory(
        dir: &Path,
        existing_names: &[String],
        skills_map: &mut HashMap<String, DiscoveredSkill>,
    ) -> Result<(), AppError> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            AppError::FileOperation(format!("读取目录失败 '{}'：{e}", dir.display()))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let skill_md_path = path.join("SKILL.md");
            if !skill_md_path.exists() {
                continue;
            }

            // 安全校验：SKILL.md 必须是普通文件（拒绝 symlink，防止读取技能目录外的任意文件）
            match skill_md_path.symlink_metadata() {
                Ok(meta) => {
                    if meta.file_type().is_symlink() {
                        eprintln!(
                            "[技能发现] SKILL.md 是符号链接，已跳过：'{}'",
                            skill_md_path.display()
                        );
                        continue;
                    }
                    if !meta.is_file() {
                        continue;
                    }
                }
                Err(_) => continue,
            }

            // 边界校验：canonicalize 后确认 SKILL.md 仍在扫描目录内
            let canonical_dir = match dir.canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let canonical_md = match skill_md_path.canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !canonical_md.starts_with(&canonical_dir) {
                eprintln!(
                    "[技能发现] SKILL.md canonicalize 后逃逸出扫描目录，已跳过：'{}'",
                    canonical_md.display()
                );
                continue;
            }

            let content = std::fs::read_to_string(&skill_md_path).map_err(|e| {
                AppError::FileOperation(format!(
                    "读取 SKILL.md 失败 '{}'：{e}",
                    skill_md_path.display()
                ))
            })?;

            match Self::parse_skill_md(&content) {
                Ok((name, description, version)) => {
                    let already_installed = existing_names.contains(&name);
                    let source_path = path.to_string_lossy().to_string();

                    skills_map.insert(
                        name.clone(),
                        DiscoveredSkill {
                            name,
                            description,
                            version,
                            skill_type: String::from("python"),
                            source_path,
                            already_installed,
                        },
                    );
                }
                Err(e) => {
                    eprintln!("[技能发现] SKILL.md 解析失败 '{}'：{e}", path.display());
                    continue;
                }
            }
        }

        Ok(())
    }

    /// 解析 SKILL.md 的 YAML frontmatter。
    ///
    /// 手动逐行解析 `---` 分隔的 YAML frontmatter，提取 name（必填）、
    /// description（必填）和 version（可选）字段。
    fn parse_skill_md(content: &str) -> Result<(String, String, Option<String>), AppError> {
        let lines: Vec<&str> = content.lines().collect();

        // 查找 frontmatter 起止位置
        if lines.is_empty() || lines[0].trim() != "---" {
            return Err(AppError::InvalidInput(String::from(
                "SKILL.md 缺少 YAML frontmatter（需以 --- 开头）",
            )));
        }

        let end_idx = lines
            .iter()
            .skip(1)
            .position(|line| line.trim() == "---")
            .map(|pos| pos + 1)
            .ok_or_else(|| {
                AppError::InvalidInput(String::from("SKILL.md frontmatter 缺少结束标记 ---"))
            })?;

        // 逐行解析 YAML 键值对
        let mut name: Option<String> = None;
        let mut description: Option<String> = None;
        let mut version: Option<String> = None;

        for line in &lines[1..end_idx] {
            let trimmed = line.trim();
            if let Some((key, value)) = trimmed.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                match key {
                    "name" => name = Some(value.to_string()),
                    "description" => description = Some(value.to_string()),
                    "version" => version = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        let name = name.ok_or_else(|| {
            AppError::InvalidInput(String::from("SKILL.md frontmatter 缺少必填字段 'name'"))
        })?;
        let description = description.ok_or_else(|| {
            AppError::InvalidInput(String::from(
                "SKILL.md frontmatter 缺少必填字段 'description'",
            ))
        })?;

        Ok((name, description, version))
    }

    /// 获取用户级技能目录路径（带 symlink 安全校验）。
    ///
    /// 通过 HOME（Unix）或 USERPROFILE（Windows）环境变量定位。
    /// 对 `~/.agents` 和 `~/.agents/skills` 逐级校验：
    /// 1. 拒绝 symlink（防止 symlink 指向白名单外目录导致扫描越权）
    /// 2. canonicalize 后确认仍在用户主目录内（边界校验）
    fn get_user_skills_dir() -> Option<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;

        let home_path = PathBuf::from(&home);

        // canonicalize HOME 本身，作为边界基准
        let canonical_home = match home_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[技能发现] 用户主目录 canonicalize 失败：{e}");
                return None;
            }
        };

        let agents_dir = home_path.join(".agents");

        // 校验 ~/.agents 不是 symlink
        if agents_dir.symlink_metadata().is_ok() {
            let meta = match agents_dir.symlink_metadata() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[技能发现] 读取 ~/.agents 元数据失败：{e}");
                    return None;
                }
            };
            if meta.file_type().is_symlink() {
                eprintln!(
                    "[技能发现] 用户级 ~/.agents 目录是符号链接，已拒绝扫描：'{}'",
                    agents_dir.display()
                );
                return None;
            }
            if !meta.is_dir() {
                return None;
            }
        } else {
            // ~/.agents 不存在
            return None;
        }

        let skills_dir = agents_dir.join("skills");

        // 校验 ~/.agents/skills 不是 symlink
        if skills_dir.symlink_metadata().is_ok() {
            let meta = match skills_dir.symlink_metadata() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[技能发现] 读取 ~/.agents/skills 元数据失败：{e}");
                    return None;
                }
            };
            if meta.file_type().is_symlink() {
                eprintln!(
                    "[技能发现] 用户级 ~/.agents/skills 目录是符号链接，已拒绝扫描：'{}'",
                    skills_dir.display()
                );
                return None;
            }
            if !meta.is_dir() {
                return None;
            }
        } else {
            return None;
        }

        // canonicalize 后确认 skills_dir 仍在用户主目录内
        let canonical_skills = match skills_dir.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[技能发现] 用户级 skills 目录 canonicalize 失败：{e}");
                return None;
            }
        };
        if !canonical_skills.starts_with(&canonical_home) {
            eprintln!(
                "[技能发现] 用户级 skills 目录 canonicalize 后逃逸出主目录：'{}'",
                canonical_skills.display()
            );
            return None;
        }

        Some(canonical_skills)
    }
}

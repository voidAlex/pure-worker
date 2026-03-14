//! 技能自动发现模块
//!
//! 扫描约定目录（`.agents/skills/`）自动发现并注册第三方技能。
//! 支持项目级和用户级两个扫描路径，项目级技能覆盖用户级同名技能。
//!
//! 遵循 Agent Skills 官方规范 (https://agentskills.io/specification)
//! 技能目录规范：每个技能子目录必须包含 `SKILL.md`，
//! 其 YAML frontmatter 包含 name、description 等字段。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::models::skill::{CreateSkillInput, SkillFrontmatter, SkillMetadata, UpdateSkillInput};
use crate::services::audit::AuditService;
use crate::services::path_whitelist::PathWhitelistService;
use crate::services::skill::SkillService;

/// 发现的技能描述结构（完整内容，用于详情展示）。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DiscoveredSkill {
    /// 技能元数据
    pub metadata: SkillMetadata,
    /// Markdown 正文内容（渐进式加载）
    pub body_content: String,
    /// 可用的脚本文件列表
    pub available_scripts: Vec<String>,
    /// 可用的引用文件列表
    pub available_references: Vec<String>,
    /// 可用的资源文件列表
    pub available_assets: Vec<String>,
    /// 入口脚本路径（如 scripts/main.py 或 run.py）
    pub entry_script: Option<String>,
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
    ) -> Result<Vec<SkillMetadata>, AppError> {
        let mut skills_map: HashMap<String, SkillMetadata> = HashMap::new();

        // 获取已注册技能列表，用于判断是否已安装
        let existing_skills = SkillService::list_skills(pool).await?;
        let existing_names: Vec<String> = existing_skills.iter().map(|s| s.name.clone()).collect();

        // 扫描用户级目录（优先级低，先扫描）
        let user_skills_dir = Self::get_user_skills_dir();
        if let Some(dir) = &user_skills_dir {
            Self::scan_directory_metadata(dir, &existing_names, &mut skills_map)?;
        }

        // 扫描项目级目录（优先级高，覆盖同名）
        match PathWhitelistService::validate_skills_dir(workspace_path) {
            Ok((_canonical_workspace, project_skills_dir)) => {
                if project_skills_dir.exists() {
                    // 扫描前二次校验（防止 TOCTOU）
                    match PathWhitelistService::validate_skills_dir(workspace_path) {
                        Ok((_cw2, verified_skills_dir)) => {
                            Self::scan_directory_metadata(
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

        let mut result: Vec<SkillMetadata> = skills_map.into_values().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    /// 发现并自动注册新技能。
    ///
    /// 扫描后对未安装的技能自动创建注册记录，返回所有发现的技能元数据。
    pub async fn discover_and_register_new(
        pool: &SqlitePool,
        workspace_path: &Path,
    ) -> Result<Vec<SkillMetadata>, AppError> {
        let mut discovered = Self::discover_skills(pool, workspace_path).await?;

        for metadata in &mut discovered {
            if metadata.already_installed {
                continue;
            }

            // 获取完整技能内容
            let skill_content = Self::load_skill_content(&metadata.source_path)?;

            let input = CreateSkillInput {
                name: metadata.name.clone(),
                version: metadata.version.clone(),
                source: Some(metadata.source_path.clone()),
                permission_scope: Some(String::from("read_only")),
                display_name: Some(metadata.name.clone()),
                description: Some(metadata.description.clone()),
                skill_type: metadata.skill_type.clone(),
                env_path: None,
                config_json: None,
                // Agent Skills 规范新增字段
                license: metadata.license.clone(),
                compatibility: metadata.compatibility.clone(),
                metadata_json: metadata.metadata_json.clone(),
                allowed_tools: metadata.allowed_tools.clone(),
                body_content: Some(skill_content.body_content),
                entry_script: skill_content.entry_script,
            };

            let created = SkillService::create_skill(pool, input).await?;
            metadata.already_installed = true;

            // Python 技能在自动发现时没有环境（env_path 为空），
            // 标记为 disabled + unhealthy，需用户手动安装环境后启用。
            if metadata.skill_type == "python" {
                let update = UpdateSkillInput {
                    display_name: None,
                    description: None,
                    permission_scope: None,
                    config_json: None,
                    status: Some(String::from("disabled")),
                    // Agent Skills 规范新增字段
                    license: None,
                    compatibility: None,
                    metadata_json: None,
                    allowed_tools: None,
                    body_content: None,
                    entry_script: None,
                };
                if let Err(e) = SkillService::update_skill(pool, &created.id, update).await {
                    eprintln!(
                        "[技能发现] 标记 Python 技能 '{}' 为 disabled 失败，回滚注册：{e}",
                        metadata.name
                    );
                    let _ = SkillService::delete_skill(pool, &created.id).await;
                    metadata.already_installed = false;
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
                        metadata.name
                    );
                    let _ = SkillService::delete_skill(pool, &created.id).await;
                    metadata.already_installed = false;
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

    /// 扫描指定目录下的技能子目录（仅加载元数据）。
    fn scan_directory_metadata(
        dir: &Path,
        existing_names: &[String],
        skills_map: &mut HashMap<String, SkillMetadata>,
    ) -> Result<(), AppError> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            AppError::FileOperation(format!("读取目录失败 '{}'：{e}", dir.display()))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // 获取目录名用于验证
            let dir_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let skill_md_path = path.join("SKILL.md");
            if !skill_md_path.exists() {
                continue;
            }

            // 安全校验：SKILL.md 必须是普通文件
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

            // 边界校验
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

            match Self::parse_skill_md_metadata(&content, &dir_name) {
                Ok(metadata) => {
                    let already_installed = existing_names.contains(&metadata.name);
                    let source_path = path.to_string_lossy().to_string();

                    skills_map.insert(
                        metadata.name.clone(),
                        SkillMetadata {
                            name: metadata.name,
                            description: metadata.description,
                            version: metadata.version,
                            license: metadata.license,
                            compatibility: metadata.compatibility,
                            metadata_json: metadata.metadata_json,
                            allowed_tools: metadata.allowed_tools,
                            source_path,
                            skill_type: String::from("python"),
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

    /// 解析 SKILL.md 的元数据（frontmatter 部分）。
    ///
    /// 使用 serde_yaml 解析完整的 YAML frontmatter，支持 Agent Skills 规范的所有字段。
    fn parse_skill_md_metadata(content: &str, dir_name: &str) -> Result<SkillMetadata, AppError> {
        // 查找 frontmatter 起止位置
        let lines: Vec<&str> = content.lines().collect();
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

        // 提取 frontmatter 内容
        let frontmatter_content = lines[1..end_idx].join("\n");

        // 使用 serde_yaml 解析
        let frontmatter: SkillFrontmatter =
            serde_yaml::from_str(&frontmatter_content).map_err(|e| {
                AppError::InvalidInput(format!("SKILL.md frontmatter YAML 解析失败：{e}"))
            })?;

        // 验证名称符合 Agent Skills 规范
        frontmatter
            .validate(Some(dir_name))
            .map_err(|e| AppError::InvalidInput(format!("技能名称验证失败：{e}")))?;

        // 将 metadata HashMap 序列化为 JSON
        let metadata_json = frontmatter
            .metadata
            .map(|m| serde_json::to_string(&m).unwrap_or_default());

        Ok(SkillMetadata {
            name: frontmatter.name,
            description: frontmatter.description,
            version: None, // version 在 metadata 中
            license: frontmatter.license,
            compatibility: frontmatter.compatibility,
            metadata_json,
            allowed_tools: frontmatter.allowed_tools,
            source_path: String::new(), // 由调用方填充
            skill_type: String::from("python"),
            already_installed: false,
        })
    }

    /// 加载技能的完整内容（渐进式加载）。
    ///
    /// 包括 frontmatter、body、scripts/、references/、assets/ 等。
    pub fn load_skill_content(source_path: &str) -> Result<DiscoveredSkill, AppError> {
        let path = Path::new(source_path);
        let skill_md_path = path.join("SKILL.md");

        if !skill_md_path.exists() {
            return Err(AppError::NotFound(format!(
                "SKILL.md 不存在：'{}'",
                skill_md_path.display()
            )));
        }

        let content = std::fs::read_to_string(&skill_md_path).map_err(|e| {
            AppError::FileOperation(format!(
                "读取 SKILL.md 失败 '{}'：{e}",
                skill_md_path.display()
            ))
        })?;

        // 解析 frontmatter 和 body
        let (frontmatter, body) = Self::parse_full_skill_md(&content)?;
        let dir_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // 验证名称
        frontmatter
            .validate(Some(&dir_name))
            .map_err(|e| AppError::InvalidInput(format!("技能名称验证失败：{e}")))?;

        // 扫描资源目录
        let available_scripts = Self::scan_subdir_files(path, "scripts")?;
        let available_references = Self::scan_subdir_files(path, "references")?;
        let available_assets = Self::scan_subdir_files(path, "assets")?;

        // 确定入口脚本
        let entry_script = Self::find_entry_script(path, &available_scripts);

        // 从 metadata HashMap 中提取 version
        let version = frontmatter
            .metadata
            .as_ref()
            .and_then(|m| m.get("version").cloned());

        // 序列化 metadata
        let metadata_json = frontmatter
            .metadata
            .map(|m| serde_json::to_string(&m).unwrap_or_default());

        let metadata = SkillMetadata {
            name: frontmatter.name,
            description: frontmatter.description,
            version,
            license: frontmatter.license,
            compatibility: frontmatter.compatibility,
            metadata_json,
            allowed_tools: frontmatter.allowed_tools,
            source_path: source_path.to_string(),
            skill_type: String::from("python"),
            already_installed: false,
        };

        Ok(DiscoveredSkill {
            metadata,
            body_content: body,
            available_scripts,
            available_references,
            available_assets,
            entry_script,
        })
    }

    /// 解析完整的 SKILL.md（frontmatter + body）。
    fn parse_full_skill_md(content: &str) -> Result<(SkillFrontmatter, String), AppError> {
        let lines: Vec<&str> = content.lines().collect();

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

        // 解析 frontmatter
        let frontmatter_content = lines[1..end_idx].join("\n");
        let frontmatter: SkillFrontmatter =
            serde_yaml::from_str(&frontmatter_content).map_err(|e| {
                AppError::InvalidInput(format!("SKILL.md frontmatter YAML 解析失败：{e}"))
            })?;

        // body 是剩下的内容
        let body = if end_idx < lines.len() {
            lines[end_idx..].join("\n")
        } else {
            String::new()
        };

        Ok((frontmatter, body))
    }

    /// 扫描子目录中的文件列表。
    fn scan_subdir_files(path: &Path, subdir: &str) -> Result<Vec<String>, AppError> {
        let subdir_path = path.join(subdir);
        if !subdir_path.exists() || !subdir_path.is_dir() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        let entries = std::fs::read_dir(&subdir_path).map_err(|e| {
            AppError::FileOperation(format!("读取目录失败 '{}'：{e}", subdir_path.display()))
        })?;

        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.is_file() {
                // 只存储相对路径
                if let Some(file_name) = file_path.file_name() {
                    files.push(format!("{}/{}", subdir, file_name.to_string_lossy()));
                }
            }
        }

        Ok(files)
    }

    /// 查找入口脚本。
    ///
    /// 优先级：
    /// 1. scripts/main.py
    /// 2. scripts/main.sh
    /// 3. scripts/main
    /// 4. run.py
    fn find_entry_script(path: &Path, available_scripts: &[String]) -> Option<String> {
        // 检查 scripts/ 下的标准入口
        let candidates = [
            "scripts/main.py",
            "scripts/main.sh",
            "scripts/main",
            "scripts/run.py",
        ];

        for candidate in &candidates {
            if available_scripts.contains(&candidate.to_string()) {
                return Some(candidate.to_string());
            }
        }

        // 回退到根目录的 run.py
        let run_py = path.join("run.py");
        if run_py.exists() {
            return Some(String::from("run.py"));
        }

        None
    }

    /// 获取用户级技能目录路径（带 symlink 安全校验）。
    fn get_user_skills_dir() -> Option<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;

        let home_path = PathBuf::from(&home);

        let canonical_home = match home_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[技能发现] 用户主目录 canonicalize 失败：{e}");
                return None;
            }
        };

        let agents_dir = home_path.join(".agents");

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
            return None;
        }

        let skills_dir = agents_dir.join("skills");

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

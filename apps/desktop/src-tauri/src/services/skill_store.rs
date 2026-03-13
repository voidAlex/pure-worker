//! 技能商店服务模块
//!
//! 提供技能的安装、卸载和列表功能。
//! 合并已安装技能与自动发现的可用技能，按名称去重（已安装优先）。
//! 支持从 Git 仓库远程安装技能（clone → 解析 SKILL.md → 创建虚拟环境 → 安装依赖 → 注册）。
//! 内置技能禁止卸载。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::models::skill::CreateSkillInput;
use crate::services::audit::AuditService;
use crate::services::skill::SkillService;
use crate::services::skill_discovery::SkillDiscoveryService;
use crate::services::uv_manager::UvManager;

/// 技能商店条目。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillStoreItem {
    /// 技能名称（唯一标识）。
    pub name: String,
    /// 技能显示名称。
    pub display_name: String,
    /// 技能描述。
    pub description: String,
    /// 技能版本（可选）。
    pub version: Option<String>,
    /// 技能来源（"local" 表示本地发现，"builtin" 表示内置）。
    pub source: String,
    /// 是否已安装。
    pub installed: bool,
    /// 技能类型（"builtin" / "python"）。
    pub skill_type: String,
}

/// 技能商店服务。
pub struct SkillStoreService;

impl SkillStoreService {
    /// 列出所有可用技能（合并已安装 + 已发现）。
    ///
    /// 按名称去重，已安装的技能优先保留。
    pub async fn list_available_skills(
        pool: &SqlitePool,
        workspace_path: &Path,
    ) -> Result<Vec<SkillStoreItem>, AppError> {
        let mut items_map: HashMap<String, SkillStoreItem> = HashMap::new();

        // 加载已发现的技能（优先级低，先添加）
        let discovered = SkillDiscoveryService::discover_skills(pool, workspace_path).await?;
        for skill in discovered {
            items_map.insert(
                skill.name.clone(),
                SkillStoreItem {
                    name: skill.name.clone(),
                    display_name: skill.name,
                    description: skill.description,
                    version: skill.version,
                    source: String::from("local"),
                    installed: skill.already_installed,
                    skill_type: skill.skill_type,
                },
            );
        }

        // 加载已安装的技能（优先级高，覆盖同名）
        let installed = SkillService::list_skills(pool).await?;
        for skill in installed {
            items_map.insert(
                skill.name.clone(),
                SkillStoreItem {
                    name: skill.name.clone(),
                    display_name: skill.display_name.unwrap_or_else(|| skill.name.clone()),
                    description: skill.description.unwrap_or_else(|| String::from("无描述")),
                    version: skill.version,
                    source: skill.source.unwrap_or_else(|| String::from("unknown")),
                    installed: true,
                    skill_type: skill.skill_type,
                },
            );
        }

        let mut result: Vec<SkillStoreItem> = items_map.into_values().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    /// 安装技能。
    ///
    /// 从已发现的技能列表中查找指定技能并注册到数据库。
    /// 已安装的技能不允许重复安装。
    pub async fn install_skill(
        pool: &SqlitePool,
        skill_name: &str,
        workspace_path: &Path,
    ) -> Result<SkillStoreItem, AppError> {
        let discovered = SkillDiscoveryService::discover_skills(pool, workspace_path).await?;
        let skill = discovered
            .iter()
            .find(|s| s.name == skill_name)
            .ok_or_else(|| AppError::NotFound(format!("未在技能目录中发现技能 '{skill_name}'")))?;

        if skill.already_installed {
            return Err(AppError::InvalidInput(format!(
                "技能 '{skill_name}' 已安装，无需重复安装"
            )));
        }

        // Python 技能：始终创建虚拟环境，有 requirements.txt 时额外安装依赖
        let env_path = if skill.skill_type == "python" {
            let skill_dir = std::path::Path::new(&skill.source_path);
            Some(Self::setup_python_env_always(&skill.name, skill_dir).await?)
        } else {
            None
        };

        let input = CreateSkillInput {
            name: skill.name.clone(),
            version: skill.version.clone(),
            source: Some(skill.source_path.clone()),
            permission_scope: Some(String::from("read_only")),
            display_name: Some(skill.name.clone()),
            description: Some(skill.description.clone()),
            skill_type: skill.skill_type.clone(),
            env_path,
            config_json: None,
        };

        let record = SkillService::create_skill(pool, input).await?;

        let detail = serde_json::json!({
            "skill_name": skill_name,
            "source_path": skill.source_path,
        });
        if let Err(e) = AuditService::log_with_detail(
            pool,
            "system",
            "install_skill",
            "skill_registry",
            Some(&record.id),
            "medium",
            false,
            Some(&detail.to_string()),
        )
        .await
        {
            eprintln!("[审计日志] 记录技能安装审计失败：{e}");
        }

        Ok(SkillStoreItem {
            name: record.name,
            display_name: record.display_name.unwrap_or_default(),
            description: record.description.unwrap_or_else(|| String::from("无描述")),
            version: record.version,
            source: record.source.unwrap_or_else(|| String::from("local")),
            installed: true,
            skill_type: record.skill_type,
        })
    }

    /// 卸载技能。
    ///
    /// 内置技能禁止卸载。通过软删除方式移除技能注册记录。
    pub async fn uninstall_skill(pool: &SqlitePool, skill_name: &str) -> Result<(), AppError> {
        let skill = SkillService::get_skill_by_name(pool, skill_name).await?;

        if skill.skill_type == "builtin" {
            return Err(AppError::PermissionDenied(String::from(
                "内置技能不允许卸载",
            )));
        }

        SkillService::delete_skill(pool, &skill.id).await?;

        let detail = serde_json::json!({
            "skill_name": skill_name,
            "skill_id": skill.id,
        });
        if let Err(e) = AuditService::log_with_detail(
            pool,
            "system",
            "uninstall_skill",
            "skill_registry",
            Some(&skill.id),
            "high",
            false,
            Some(&detail.to_string()),
        )
        .await
        {
            eprintln!("[审计日志] 记录技能卸载审计失败：{e}");
        }

        Ok(())
    }

    /// 从 Git 仓库远程安装技能。
    ///
    /// 完整流程：
    /// 1. 校验 Git URL 来源白名单
    /// 2. 浅克隆仓库到本地技能目录（`{workspace}/.agents/skills/{repo_name}`）
    /// 3. 解析 SKILL.md frontmatter 获取技能元信息
    /// 4. 若存在 requirements.txt，通过 uv 创建虚拟环境并安装依赖
    /// 5. 注册技能到数据库并写入审计日志
    pub async fn install_from_git(
        pool: &SqlitePool,
        git_url: &str,
        workspace_path: &Path,
    ) -> Result<SkillStoreItem, AppError> {
        // 校验 Git URL 非空
        let git_url = git_url.trim();
        if git_url.is_empty() {
            return Err(AppError::InvalidInput(String::from("Git URL 不能为空")));
        }

        // 校验来源白名单（仅允许 github.com 和 gitee.com）
        Self::validate_git_url(git_url)?;

        // 从 URL 中提取仓库名称作为技能目录名
        let repo_name = Self::extract_repo_name(git_url)?;

        // 检查是否已安装同名技能
        if SkillService::get_skill_by_name(pool, &repo_name)
            .await
            .is_ok()
        {
            return Err(AppError::InvalidInput(format!(
                "技能 '{repo_name}' 已存在，如需重新安装请先卸载"
            )));
        }

        // 构建目标目录：{workspace}/.agents/skills/{repo_name}
        let skills_dir = workspace_path.join(".agents").join("skills");
        tokio::fs::create_dir_all(&skills_dir)
            .await
            .map_err(|e| AppError::FileOperation(format!("创建技能目录失败：{e}")))?;

        let target_dir = skills_dir.join(&repo_name);
        if target_dir.exists() {
            return Err(AppError::InvalidInput(format!(
                "目标目录已存在：'{}'，请先手动删除或选择其他名称",
                target_dir.display()
            )));
        }

        // 浅克隆仓库（depth=1 减少下载量）
        Self::git_clone(git_url, &target_dir).await?;

        // clone 成功后的所有步骤统一走 cleanup-on-error：
        // 任何失败都清理 target_dir，避免半成品目录残留阻塞重试。
        match Self::post_clone_setup(pool, git_url, &target_dir).await {
            Ok(item) => Ok(item),
            Err(e) => {
                let _ = tokio::fs::remove_dir_all(&target_dir).await;
                Err(e)
            }
        }
    }

    /// clone 成功后的安装步骤（解析 SKILL.md → 创建 venv → 安装依赖 → 注册 DB → 审计）。
    ///
    /// 此方法由 `install_from_git` 调用，任何失败由调用方统一清理 target_dir。
    async fn post_clone_setup(
        pool: &SqlitePool,
        git_url: &str,
        target_dir: &Path,
    ) -> Result<SkillStoreItem, AppError> {
        let skill_md_path = target_dir.join("SKILL.md");
        if !skill_md_path.exists() {
            return Err(AppError::InvalidInput(format!(
                "Git 仓库 '{git_url}' 缺少 SKILL.md 文件，不符合技能目录规范"
            )));
        }

        let skill_md_content = tokio::fs::read_to_string(&skill_md_path)
            .await
            .map_err(|e| AppError::FileOperation(format!("读取 SKILL.md 失败：{e}")))?;

        let (skill_name, description, version) = Self::parse_skill_md_content(&skill_md_content)?;

        // Python 技能始终创建 venv（确保 env_path 非空），仅在有 requirements.txt 时安装依赖
        let env_path = Self::setup_python_env_always(&skill_name, target_dir).await?;

        let input = CreateSkillInput {
            name: skill_name.clone(),
            version: version.clone(),
            source: Some(target_dir.to_string_lossy().to_string()),
            permission_scope: Some(String::from("read_only")),
            display_name: Some(skill_name.clone()),
            description: Some(description.clone()),
            skill_type: String::from("python"),
            env_path: Some(env_path),
            config_json: None,
        };

        let record = SkillService::create_skill(pool, input).await?;

        let detail = serde_json::json!({
            "skill_name": skill_name,
            "git_url": git_url,
            "target_dir": target_dir.to_string_lossy(),
            "version": version,
        });
        if let Err(e) = AuditService::log_with_detail(
            pool,
            "system",
            "install_skill_from_git",
            "skill_registry",
            Some(&record.id),
            "high",
            false,
            Some(&detail.to_string()),
        )
        .await
        {
            eprintln!("[审计日志] 记录 Git 技能安装审计失败：{e}");
        }

        Ok(SkillStoreItem {
            name: record.name,
            display_name: record.display_name.unwrap_or_default(),
            description: record.description.unwrap_or_else(|| String::from("无描述")),
            version: record.version,
            source: String::from("git"),
            installed: true,
            skill_type: record.skill_type,
        })
    }

    /// 校验 Git URL 来源是否在白名单内。
    ///
    /// 仅允许 github.com 和 gitee.com 域名，使用前缀匹配防止绕过
    /// （如 `evil-github.com` 不会通过 `starts_with("https://github.com/")` 校验）。
    fn validate_git_url(url: &str) -> Result<(), AppError> {
        let allowed_prefixes = [
            "https://github.com/",
            "https://gitee.com/",
            "git@github.com:",
            "git@gitee.com:",
        ];

        let is_allowed = allowed_prefixes
            .iter()
            .any(|prefix| url.starts_with(prefix));

        if !is_allowed {
            return Err(AppError::PermissionDenied(format!(
                "Git 来源不在白名单内，仅允许 github.com 和 gitee.com（HTTPS 或 SSH 协议）。收到：{url}"
            )));
        }

        Ok(())
    }

    /// 从 Git URL 中提取仓库名称。
    ///
    /// 支持 HTTPS（`https://github.com/user/repo.git`）和
    /// SSH（`git@github.com:user/repo.git`）格式。
    fn extract_repo_name(url: &str) -> Result<String, AppError> {
        let name = url
            .rsplit('/')
            .next()
            .or_else(|| url.rsplit(':').next())
            .unwrap_or(url);

        let name = name.strip_suffix(".git").unwrap_or(name);
        let name = name.trim();

        if name.is_empty() {
            return Err(AppError::InvalidInput(format!(
                "无法从 Git URL 提取仓库名称：{url}"
            )));
        }

        Ok(name.to_string())
    }

    /// git clone 超时时间（秒）。
    const GIT_CLONE_TIMEOUT_SECS: u64 = 120;

    /// 执行 git clone --depth 1 浅克隆（带超时保护和显式进程终止）。
    async fn git_clone(url: &str, target: &PathBuf) -> Result<(), AppError> {
        use tokio::process::Command;

        // kill_on_drop 确保超时/异常时 git 子进程被显式终止
        let child = Command::new("git")
            .args(["clone", "--depth", "1", url])
            .arg(target)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                AppError::ExternalService(format!(
                    "执行 git clone 失败（请确认系统已安装 git）：{e}"
                ))
            })?;

        let timeout_duration = std::time::Duration::from_secs(Self::GIT_CLONE_TIMEOUT_SECS);

        match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
            Ok(result) => {
                let output = result.map_err(|e| {
                    AppError::ExternalService(format!("等待 git clone 完成失败：{e}"))
                })?;

                if !output.status.success() {
                    // 克隆失败时清理可能已创建的半成品目录
                    let _ = tokio::fs::remove_dir_all(target).await;
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(AppError::ExternalService(format!(
                        "git clone 失败：{stderr}"
                    )));
                }

                Ok(())
            }
            Err(_) => {
                // 超时：wait_with_output() 的 future 被 drop，
                // tokio::process::Child 的 Drop 实现会自动发送 kill 信号终止子进程。
                // 显式清理超时产生的半成品目录，避免残留
                let _ = tokio::fs::remove_dir_all(target).await;
                Err(AppError::ExternalService(format!(
                    "git clone 超时（{} 秒），已清理半成品目录，请检查网络连接或仓库地址",
                    Self::GIT_CLONE_TIMEOUT_SECS
                )))
            }
        }
    }

    /// 解析 SKILL.md 的 YAML frontmatter。
    ///
    /// 复用 `SkillDiscoveryService` 中相同的逐行解析逻辑，
    /// 提取 name（必填）、description（必填）和 version（可选）。
    fn parse_skill_md_content(content: &str) -> Result<(String, String, Option<String>), AppError> {
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

    /// 为 Python 技能始终创建虚拟环境，有 requirements.txt 时额外安装依赖。
    ///
    /// 确保所有 Python 技能都有 env_path，避免"安装成功但无法执行"的问题。
    async fn setup_python_env_always(
        skill_name: &str,
        skill_dir: &Path,
    ) -> Result<String, AppError> {
        let env_path = UvManager::create_skill_env(skill_name, None).await?;

        let requirements_path = skill_dir.join("requirements.txt");
        if requirements_path.exists() {
            UvManager::install_skill_deps(&env_path, &requirements_path.to_string_lossy()).await?;
        }

        Ok(env_path)
    }
}

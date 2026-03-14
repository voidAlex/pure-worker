//! 技能注册服务模块
//!
//! 提供技能注册信息的增删改查与健康检查能力。
//! 遵循 Agent Skills 官方规范 (https://agentskills.io/specification)

use chrono::Utc;
use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::skill::{CreateSkillInput, SkillHealthResult, SkillRecord, UpdateSkillInput};
use crate::services::audit::AuditService;

/// 校验技能名称是否符合 Agent Skills 官方规范。
///
/// 规范要求：
/// - 1-64 个字符
/// - 只能包含小写字母 a-z、数字 0-9 和连字符 -
/// - 不能以连字符开头或结尾
/// - 不能包含连续的连字符 (--)
fn validate_skill_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::InvalidInput(String::from("技能名称不能为空")));
    }

    if name.len() > 64 {
        return Err(AppError::InvalidInput(format!(
            "技能名称 '{}' 长度不合法：必须为 1-64 个字符，当前为 {} 个字符",
            name,
            name.len()
        )));
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err(AppError::InvalidInput(format!(
            "技能名称 '{}' 不能以连字符开头或结尾",
            name
        )));
    }

    if name.contains("--") {
        return Err(AppError::InvalidInput(format!(
            "技能名称 '{}' 不能包含连续的连字符（--）",
            name
        )));
    }

    for ch in name.chars() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
            return Err(AppError::InvalidInput(format!(
                "技能名称 '{}' 包含非法字符 '{}'：只能使用小写字母 a-z、数字 0-9 和连字符 -",
                name, ch
            )));
        }
    }

    Ok(())
}

/// 校验 Python 技能 source 路径落在合法的技能目录下。
fn validate_python_source_path(source: &str) -> Result<(), AppError> {
    let source_path = Path::new(source);
    let canonical = source_path.canonicalize().map_err(|e| {
        AppError::InvalidInput(format!(
            "Python 技能 source 路径不存在或无法解析：'{source}' — {e}"
        ))
    })?;

    if !is_under_agents_skills_dir(&canonical) {
        return Err(AppError::InvalidInput(format!(
            "Python 技能 source 必须位于 .agents/skills/ 目录下，当前规范路径：'{}'",
            canonical.display()
        )));
    }

    if !is_under_allowed_roots(&canonical) {
        return Err(AppError::InvalidInput(format!(
            "Python 技能 source 必须位于用户目录或临时目录内，当前规范路径：'{}'",
            canonical.display()
        )));
    }

    Ok(())
}

/// 判断路径是否位于 .agents/skills/ 目录结构下。
fn is_under_agents_skills_dir(path: &Path) -> bool {
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        if let std::path::Component::Normal(name) = component {
            if name == ".agents" {
                if let Some(std::path::Component::Normal(next)) = components.peek() {
                    if *next == "skills" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 校验路径是否位于允许的根目录内。
fn is_under_allowed_roots(path: &Path) -> bool {
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    };

    if let Some(home_str) = home {
        if let Ok(home_path) = Path::new(&home_str).canonicalize() {
            if path.starts_with(&home_path) {
                return true;
            }
        }
    }

    if let Ok(canonical_temp) = std::env::temp_dir().canonicalize() {
        if path.starts_with(&canonical_temp) {
            return true;
        }
    }

    false
}

/// 校验 Python 技能 env_path 落在 ~/.pureworker/skill-envs/ 下。
fn validate_python_env_path(env_path: &str) -> Result<(), AppError> {
    let env_path_obj = Path::new(env_path);
    let canonical_env = env_path_obj.canonicalize().map_err(|e| {
        AppError::InvalidInput(format!(
            "Python 技能 env_path 路径不存在或无法解析：'{env_path}' — {e}"
        ))
    })?;

    let home = if cfg!(windows) {
        std::env::var("USERPROFILE")
    } else {
        std::env::var("HOME")
    }
    .map_err(|_| AppError::Config(String::from("未找到用户主目录环境变量")))?;

    let expected_base = Path::new(&home).join(".pureworker").join("skill-envs");

    let pureworker_dir = Path::new(&home).join(".pureworker");
    if pureworker_dir.exists() {
        let meta = pureworker_dir.symlink_metadata().map_err(|e| {
            AppError::InvalidInput(format!("无法读取 ~/.pureworker 目录元数据：{e}"))
        })?;
        if meta.file_type().is_symlink() {
            return Err(AppError::PermissionDenied(String::from(
                "~/.pureworker 是符号链接，拒绝校验 env_path",
            )));
        }
    }
    if expected_base.exists() {
        let meta = expected_base.symlink_metadata().map_err(|e| {
            AppError::InvalidInput(format!("无法读取 ~/.pureworker/skill-envs 目录元数据：{e}"))
        })?;
        if meta.file_type().is_symlink() {
            return Err(AppError::PermissionDenied(String::from(
                "~/.pureworker/skill-envs 是符号链接，拒绝校验 env_path",
            )));
        }
    }

    let canonical_base = expected_base
        .canonicalize()
        .map_err(|e| AppError::InvalidInput(format!("技能环境根路径不存在或无法解析：{e}")))?;

    if !canonical_env.starts_with(&canonical_base) {
        return Err(AppError::InvalidInput(format!(
            "Python 技能 env_path 必须位于 ~/.pureworker/skill-envs/ 下，当前规范路径：'{}'",
            canonical_env.display()
        )));
    }
    Ok(())
}

/// 技能注册服务。
pub struct SkillService;

impl SkillService {
    /// SQL 查询字段列表（与 SkillRecord 结构对应）。
    const SELECT_FIELDS: &'static str = "id, name, version, source, permission_scope, status, is_deleted, created_at, display_name, description, skill_type, env_path, config_json, updated_at, health_status, last_health_check, license, compatibility, metadata_json, allowed_tools, body_content, entry_script";

    /// 列出所有未删除技能。
    pub async fn list_skills(pool: &SqlitePool) -> Result<Vec<SkillRecord>, AppError> {
        let sql = format!(
            "SELECT {} FROM skill_registry WHERE is_deleted = 0 ORDER BY created_at DESC",
            Self::SELECT_FIELDS
        );
        let items = sqlx::query_as::<_, SkillRecord>(&sql)
            .fetch_all(pool)
            .await?;

        Ok(items)
    }

    /// 根据 ID 获取技能。
    pub async fn get_skill(pool: &SqlitePool, id: &str) -> Result<SkillRecord, AppError> {
        let sql = format!(
            "SELECT {} FROM skill_registry WHERE id = ? AND is_deleted = 0",
            Self::SELECT_FIELDS
        );
        let item = sqlx::query_as::<_, SkillRecord>(&sql)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("技能不存在：{id}")))?;

        Ok(item)
    }

    /// 根据名称获取技能（取最新版本）。
    pub async fn get_skill_by_name(pool: &SqlitePool, name: &str) -> Result<SkillRecord, AppError> {
        let sql = format!(
            "SELECT {} FROM skill_registry WHERE name = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT 1",
            Self::SELECT_FIELDS
        );
        let item = sqlx::query_as::<_, SkillRecord>(&sql)
            .bind(name)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("技能不存在：{name}")))?;

        Ok(item)
    }

    /// 创建技能。
    pub async fn create_skill(
        pool: &SqlitePool,
        input: CreateSkillInput,
    ) -> Result<SkillRecord, AppError> {
        if input.name.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("name 不能为空")));
        }
        if input.skill_type.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("skill_type 不能为空")));
        }

        validate_skill_name(&input.name)?;

        match input.skill_type.as_str() {
            "builtin" => {
                if input.source.is_some() || input.env_path.is_some() {
                    return Err(AppError::InvalidInput(String::from(
                        "内置技能不允许设置 source 或 env_path",
                    )));
                }
            }
            "python" => {
                let source = input.source.as_deref().unwrap_or("").trim();
                if source.is_empty() {
                    return Err(AppError::InvalidInput(String::from(
                        "Python 技能必须提供 source（技能仓库目录路径）",
                    )));
                }
                validate_python_source_path(source)?;

                if let Some(ref ep) = input.env_path {
                    let ep_trimmed = ep.trim();
                    if !ep_trimmed.is_empty() {
                        validate_python_env_path(ep_trimmed)?;
                    }
                }
            }
            other => {
                return Err(AppError::InvalidInput(format!(
                    "不支持的技能类型：'{other}'，仅支持 builtin 和 python"
                )));
            }
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO skill_registry (id, name, version, source, permission_scope, status, is_deleted, created_at, display_name, description, skill_type, env_path, config_json, updated_at, health_status, last_health_check, license, compatibility, metadata_json, allowed_tools, body_content, entry_script) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, 'unknown', NULL, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&input.version)
        .bind(&input.source)
        .bind(&input.permission_scope)
        .bind("enabled")
        .bind(&now)
        .bind(&input.display_name)
        .bind(&input.description)
        .bind(&input.skill_type)
        .bind(&input.env_path)
        .bind(&input.config_json)
        .bind(&now)
        .bind(&input.license)
        .bind(&input.compatibility)
        .bind(&input.metadata_json)
        .bind(&input.allowed_tools)
        .bind(&input.body_content)
        .bind(&input.entry_script)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_skill",
            "skill_registry",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_skill(pool, &id).await
    }

    /// 更新技能。
    pub async fn update_skill(
        pool: &SqlitePool,
        id: &str,
        input: UpdateSkillInput,
    ) -> Result<SkillRecord, AppError> {
        Self::get_skill(pool, id).await?;

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE skill_registry SET display_name = COALESCE(?, display_name), description = COALESCE(?, description), permission_scope = COALESCE(?, permission_scope), config_json = COALESCE(?, config_json), status = COALESCE(?, status), license = COALESCE(?, license), compatibility = COALESCE(?, compatibility), metadata_json = COALESCE(?, metadata_json), allowed_tools = COALESCE(?, allowed_tools), body_content = COALESCE(?, body_content), entry_script = COALESCE(?, entry_script), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.display_name)
        .bind(&input.description)
        .bind(&input.permission_scope)
        .bind(&input.config_json)
        .bind(&input.status)
        .bind(&input.license)
        .bind(&input.compatibility)
        .bind(&input.metadata_json)
        .bind(&input.allowed_tools)
        .bind(&input.body_content)
        .bind(&input.entry_script)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("技能不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "update_skill",
            "skill_registry",
            Some(id),
            "medium",
            false,
        )
        .await?;

        Self::get_skill(pool, id).await
    }

    /// 软删除技能。
    pub async fn delete_skill(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE skill_registry SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("技能不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_skill",
            "skill_registry",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 检查技能健康状态并写回数据库。
    pub async fn check_skill_health(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<SkillHealthResult, AppError> {
        let skill = Self::get_skill(pool, id).await?;
        let checked_at = Utc::now().to_rfc3339();

        let (health_status, message) = match skill.skill_type.as_str() {
            "builtin" => (String::from("healthy"), String::from("内置技能始终可用")),
            "python" => match skill.env_path.as_deref() {
                Some(path) if Path::new(path).exists() => {
                    (String::from("healthy"), String::from("Python 环境路径存在"))
                }
                Some(path) => (
                    String::from("unhealthy"),
                    format!("Python 环境路径不存在：{path}"),
                ),
                None => (
                    String::from("unhealthy"),
                    String::from("Python 技能缺少 env_path 配置"),
                ),
            },
            _ => (
                String::from("unknown"),
                String::from("当前技能类型尚未定义健康检查逻辑"),
            ),
        };

        sqlx::query(
            "UPDATE skill_registry SET health_status = ?, last_health_check = ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&health_status)
        .bind(&checked_at)
        .bind(&checked_at)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(SkillHealthResult {
            name: skill.name,
            health_status,
            message,
            checked_at,
        })
    }
}

//! 技能注册服务模块
//!
//! 提供技能注册信息的增删改查与健康检查能力。

use chrono::Utc;
use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::skill::{CreateSkillInput, SkillHealthResult, SkillRecord, UpdateSkillInput};
use crate::services::audit::AuditService;

/// 校验技能名称，仅允许 `[A-Za-z0-9._-]`，防止目录穿越和路径注入。
fn validate_skill_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::InvalidInput(String::from("技能名称不能为空")));
    }
    if name == "." || name == ".." {
        return Err(AppError::InvalidInput(format!("技能名称不合法：'{name}'")));
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
    if !valid {
        return Err(AppError::InvalidInput(format!(
            "技能名称仅允许字母、数字、点、下划线和连字符：'{name}'"
        )));
    }
    Ok(())
}

/// 校验 Python 技能 source 路径落在合法的技能目录下（canonicalize 解析符号链接和 `..`）。
///
/// 合法路径必须满足：路径存在，且 canonicalize 后位于某个 `.agents/skills/` 根目录下。
fn validate_python_source_path(source: &str) -> Result<(), AppError> {
    let source_path = Path::new(source);
    let canonical = source_path.canonicalize().map_err(|e| {
        AppError::InvalidInput(format!(
            "Python 技能 source 路径不存在或无法解析：'{source}' — {e}"
        ))
    })?;

    let sep = std::path::MAIN_SEPARATOR;
    let agents_skills = format!("{sep}.agents{sep}skills{sep}");
    let canonical_str = canonical.to_string_lossy();
    if !canonical_str.contains(&agents_skills) {
        return Err(AppError::InvalidInput(format!(
            "Python 技能 source 必须位于 .agents/skills/ 目录下，当前规范路径：'{}'",
            canonical.display()
        )));
    }
    Ok(())
}

/// 校验 Python 技能 env_path 落在 ~/.pureworker/skill-envs/ 下（canonicalize 解析符号链接）。
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
    /// 列出所有未删除技能。
    pub async fn list_skills(pool: &SqlitePool) -> Result<Vec<SkillRecord>, AppError> {
        let items = sqlx::query_as::<_, SkillRecord>(
            "SELECT id, name, version, source, permission_scope, status, is_deleted, created_at, display_name, description, skill_type, env_path, config_json, updated_at, health_status, last_health_check FROM skill_registry WHERE is_deleted = 0 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 根据 ID 获取技能。
    pub async fn get_skill(pool: &SqlitePool, id: &str) -> Result<SkillRecord, AppError> {
        let item = sqlx::query_as::<_, SkillRecord>(
            "SELECT id, name, version, source, permission_scope, status, is_deleted, created_at, display_name, description, skill_type, env_path, config_json, updated_at, health_status, last_health_check FROM skill_registry WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("技能不存在：{id}")))?;

        Ok(item)
    }

    /// 根据名称获取技能（取最新版本）。
    pub async fn get_skill_by_name(pool: &SqlitePool, name: &str) -> Result<SkillRecord, AppError> {
        let item = sqlx::query_as::<_, SkillRecord>(
            "SELECT id, name, version, source, permission_scope, status, is_deleted, created_at, display_name, description, skill_type, env_path, config_json, updated_at, health_status, last_health_check FROM skill_registry WHERE name = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT 1",
        )
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

                // env_path 允许为空（自动发现时尚未创建虚拟环境，后续安装时补全）
                // 有值时必须通过路径边界校验
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
            "INSERT INTO skill_registry (id, name, version, source, permission_scope, status, is_deleted, created_at, display_name, description, skill_type, env_path, config_json, updated_at, health_status, last_health_check) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, 'unknown', NULL)",
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
            "UPDATE skill_registry SET display_name = COALESCE(?, display_name), description = COALESCE(?, description), permission_scope = COALESCE(?, permission_scope), config_json = COALESCE(?, config_json), status = COALESCE(?, status), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.display_name)
        .bind(&input.description)
        .bind(&input.permission_scope)
        .bind(&input.config_json)
        .bind(&input.status)
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

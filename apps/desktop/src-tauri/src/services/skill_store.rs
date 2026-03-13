//! 技能商店服务模块
//!
//! 提供技能的安装、卸载和列表功能。
//! 合并已安装技能与自动发现的可用技能，按名称去重（已安装优先）。
//! 内置技能禁止卸载。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;

use crate::error::AppError;
use crate::models::skill::CreateSkillInput;
use crate::services::audit::AuditService;
use crate::services::skill::SkillService;
use crate::services::skill_discovery::SkillDiscoveryService;

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
                    name: skill.name,
                    display_name: skill.description.clone(),
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

        let input = CreateSkillInput {
            name: skill.name.clone(),
            version: skill.version.clone(),
            source: Some(skill.source_path.clone()),
            permission_scope: Some(String::from("read_only")),
            display_name: Some(skill.name.clone()),
            description: Some(skill.description.clone()),
            skill_type: skill.skill_type.clone(),
            config_json: None,
        };

        let record = SkillService::create_skill(pool, input).await?;

        let detail = serde_json::json!({
            "skill_name": skill_name,
            "source_path": skill.source_path,
        });
        let _ = AuditService::log_with_detail(
            pool,
            "system",
            "install_skill",
            "skill_registry",
            Some(&record.id),
            "medium",
            false,
            Some(&detail.to_string()),
        )
        .await;

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
        let _ = AuditService::log_with_detail(
            pool,
            "system",
            "uninstall_skill",
            "skill_registry",
            Some(&skill.id),
            "high",
            false,
            Some(&detail.to_string()),
        )
        .await;

        Ok(())
    }
}

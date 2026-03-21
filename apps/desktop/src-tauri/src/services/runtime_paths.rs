use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::error::AppError;

const RUNTIME_ROOT_DIR: &str = ".pureworker";
const DB_SUBDIR: &str = "db";
const LOG_SUBDIR: &str = "logs";
const SKILLS_SUBDIR: &str = "skills";
const BUILTIN_SKILLS_SUBDIR: &str = "builtin";
const WORKSPACE_SKILLS_SUBDIR: &str = ".agents/skills";
const DB_FILE_NAME: &str = "pureworker.db";
const RUNTIME_CONFIG_FILE: &str = "runtime-paths.json";
const WORK_FILES_DIRS: [&str; 5] = ["students", "archives", "templates", "exports", "imports"];

#[derive(Debug, Serialize, Deserialize)]
struct RuntimePathsConfig {
    workspace_root: String,
}

#[derive(Debug, Serialize)]
struct BuiltinSkillMeta {
    name: &'static str,
    display_name: &'static str,
    version: &'static str,
}

const BUILTIN_SKILLS: [BuiltinSkillMeta; 5] = [
    BuiltinSkillMeta {
        name: "office.read_write",
        display_name: "文档读取",
        version: "1.0.0",
    },
    BuiltinSkillMeta {
        name: "ocr.extract",
        display_name: "OCR 文字提取",
        version: "1.0.0",
    },
    BuiltinSkillMeta {
        name: "image.preprocess",
        display_name: "图像预处理",
        version: "1.0.0",
    },
    BuiltinSkillMeta {
        name: "math.compute",
        display_name: "数学计算",
        version: "1.0.0",
    },
    BuiltinSkillMeta {
        name: "export.render",
        display_name: "导出渲染",
        version: "1.0.0",
    },
];

pub fn normalize_workspace_setting_value(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if let Ok(parsed) = serde_json::from_str::<String>(trimmed) {
        let clean = parsed.trim();
        if !clean.is_empty() {
            return PathBuf::from(clean);
        }
    }

    let without_quotes = trimmed.trim_matches('"').trim();
    PathBuf::from(without_quotes)
}

pub fn resolve_workspace_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    if let Some(path) = load_workspace_path_from_config(app_handle)? {
        return Ok(path);
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Config(format!("获取应用数据目录失败：{}", e)))?;
    Ok(app_data_dir.join("workspace"))
}

pub fn persist_workspace_path(
    app_handle: &tauri::AppHandle,
    workspace_path: &Path,
) -> Result<(), AppError> {
    let config_path = runtime_config_path(app_handle)?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::FileOperation(format!("创建运行时配置目录失败：{}", e)))?;
    }

    let config = RuntimePathsConfig {
        workspace_root: workspace_path.to_string_lossy().to_string(),
    };

    let content = serde_json::to_vec_pretty(&config)
        .map_err(|e| AppError::Config(format!("序列化运行时路径配置失败：{}", e)))?;
    std::fs::write(&config_path, content)
        .map_err(|e| AppError::FileOperation(format!("写入运行时路径配置失败：{}", e)))
}

pub fn runtime_root_dir(workspace_path: &Path) -> PathBuf {
    workspace_path.join(RUNTIME_ROOT_DIR)
}

pub fn database_file_path(workspace_path: &Path) -> PathBuf {
    runtime_root_dir(workspace_path)
        .join(DB_SUBDIR)
        .join(DB_FILE_NAME)
}

pub fn log_dir_path(workspace_path: &Path) -> PathBuf {
    runtime_root_dir(workspace_path).join(LOG_SUBDIR)
}

pub fn workspace_skills_dir(workspace_path: &Path) -> PathBuf {
    workspace_path.join(WORKSPACE_SKILLS_SUBDIR)
}

pub fn builtin_skills_dir(workspace_path: &Path) -> PathBuf {
    runtime_root_dir(workspace_path)
        .join(SKILLS_SUBDIR)
        .join(BUILTIN_SKILLS_SUBDIR)
}

pub fn ensure_workspace_layout(workspace_path: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(workspace_path).map_err(|e| {
        AppError::FileOperation(format!(
            "创建工作目录失败：{}，路径：{}",
            e,
            workspace_path.display()
        ))
    })?;

    let runtime_root = runtime_root_dir(workspace_path);
    let db_dir = runtime_root.join(DB_SUBDIR);
    let logs_dir = runtime_root.join(LOG_SUBDIR);
    let builtin_dir = builtin_skills_dir(workspace_path);
    let skills_dir = workspace_skills_dir(workspace_path);

    for dir in [db_dir, logs_dir, builtin_dir.clone(), skills_dir] {
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::FileOperation(format!("创建目录失败：{}，路径：{}", e, dir.display()))
        })?;
    }

    for dir_name in WORK_FILES_DIRS {
        let dir = workspace_path.join(dir_name);
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::FileOperation(format!("创建目录失败：{}，路径：{}", e, dir.display()))
        })?;
    }

    write_builtin_skills_manifest(&builtin_dir)
}

fn runtime_config_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Config(format!("获取应用配置目录失败：{}", e)))?;
    Ok(app_config_dir.join(RUNTIME_CONFIG_FILE))
}

fn load_workspace_path_from_config(
    app_handle: &tauri::AppHandle,
) -> Result<Option<PathBuf>, AppError> {
    let config_path = runtime_config_path(app_handle)?;
    if !config_path.exists() {
        return Ok(None);
    }

    let raw = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::FileOperation(format!("读取运行时路径配置失败：{}", e)))?;
    let config: RuntimePathsConfig = serde_json::from_str(&raw)
        .map_err(|e| AppError::Config(format!("解析运行时路径配置失败：{}", e)))?;

    let trimmed = config.workspace_root.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(trimmed)))
}

fn write_builtin_skills_manifest(builtin_dir: &Path) -> Result<(), AppError> {
    let manifest_path = builtin_dir.join("builtin-skills.json");
    if !manifest_path.exists() {
        let content = serde_json::to_vec_pretty(&BUILTIN_SKILLS)
            .map_err(|e| AppError::Config(format!("序列化内置技能清单失败：{}", e)))?;
        std::fs::write(&manifest_path, content)
            .map_err(|e| AppError::FileOperation(format!("写入内置技能清单失败：{}", e)))?;
    }

    let readme_path = builtin_dir.join("README.md");
    if !readme_path.exists() {
        let readme = "# 系统内置 Skills\n\n此目录用于落盘展示系统内置 Skills 清单。\n\n- `builtin-skills.json`：内置技能元数据（名称、展示名、版本）\n- 运行时实现位于 Rust 后端，不依赖此目录内的脚本执行\n";
        std::fs::write(&readme_path, readme)
            .map_err(|e| AppError::FileOperation(format!("写入内置技能说明失败：{}", e)))?;
    }

    Ok(())
}

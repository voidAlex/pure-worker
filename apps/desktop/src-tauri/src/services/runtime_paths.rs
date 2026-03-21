use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::error::AppError;

const RUNTIME_ROOT_DIR: &str = ".pureworker";
const DB_SUBDIR: &str = "db";
const LOG_DIR_NAME: &str = "logs";
const SKILLS_SUBDIR: &str = "skills";
const BUILTIN_SKILLS_SUBDIR: &str = "builtin";
const WORKSPACE_SKILLS_SUBDIR: &str = ".agents/skills";
const DB_FILE_NAME: &str = "pureworker.db";
const STARTUP_LOG_FILE_NAME: &str = "startup.log";
const RUNTIME_CONFIG_FILE: &str = "runtime-paths.json";
const WORK_FILES_DIRS: [&str; 5] = ["students", "archives", "templates", "exports", "imports"];

#[derive(Debug, Serialize, Deserialize)]
struct RuntimePathsConfig {
    workspace_root: String,
}

#[derive(Debug, Serialize)]
struct BuiltinSkillMeta {
    dir_name: &'static str,
    name: &'static str,
    display_name: &'static str,
    version: &'static str,
    description: &'static str,
}

const BUILTIN_SKILLS: [BuiltinSkillMeta; 5] = [
    BuiltinSkillMeta {
        dir_name: "office-read-write",
        name: "office.read_write",
        display_name: "文档读取",
        version: "1.0.0",
        description: "读取 Office 文档（Word、Excel），支持内容提取和结构化输出。",
    },
    BuiltinSkillMeta {
        dir_name: "ocr-extract",
        name: "ocr.extract",
        display_name: "OCR 文字提取",
        version: "1.0.0",
        description: "从图片中提取文字内容，支持作业、试卷等教育场景文档识别。",
    },
    BuiltinSkillMeta {
        dir_name: "image-preprocess",
        name: "image.preprocess",
        display_name: "图像预处理",
        version: "1.0.0",
        description: "图像预处理工具，支持裁剪、旋转、缩放、灰度转换等基础操作。",
    },
    BuiltinSkillMeta {
        dir_name: "math-compute",
        name: "math.compute",
        display_name: "数学计算",
        version: "1.0.0",
        description: "数学表达式计算引擎，支持四则运算、函数计算和公式求值。",
    },
    BuiltinSkillMeta {
        dir_name: "export-render",
        name: "export.render",
        display_name: "导出渲染",
        version: "1.0.0",
        description: "将结构化数据渲染为 Word/Excel 文档并导出。",
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
    match load_workspace_path_from_config(app_handle) {
        Ok(Some(path)) => return Ok(path),
        Ok(None) => {}
        Err(e) => {
            eprintln!("[RuntimePaths] 工作目录配置读取失败，回退到默认路径：{}", e);
        }
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
    workspace_path.join(LOG_DIR_NAME)
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
    let logs_dir = log_dir_path(workspace_path);
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

    ensure_startup_log_file(workspace_path)?;
    write_builtin_skills_manifest(&builtin_dir)?;
    unpack_builtin_skill_packages(&builtin_dir)
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
    let content = serde_json::to_vec_pretty(&BUILTIN_SKILLS)
        .map_err(|e| AppError::Config(format!("序列化内置技能清单失败：{}", e)))?;
    std::fs::write(&manifest_path, content)
        .map_err(|e| AppError::FileOperation(format!("写入内置技能清单失败：{}", e)))?;

    let readme_path = builtin_dir.join("README.md");
    if !readme_path.exists() {
        let readme = "# 系统内置 Skills\n\n此目录包含系统内置 Skills 的文件化插件包。\n\n- `builtin-skills.json`：内置技能元数据清单\n- `<skill-dir>/SKILL.md`：插件描述\n- `<skill-dir>/package.json`：插件包元数据\n\n说明：运行时执行仍由 Rust 内置实现负责。\n";
        std::fs::write(&readme_path, readme)
            .map_err(|e| AppError::FileOperation(format!("写入内置技能说明失败：{}", e)))?;
    }

    Ok(())
}

fn ensure_startup_log_file(workspace_path: &Path) -> Result<(), AppError> {
    let file_path = log_dir_path(workspace_path).join(STARTUP_LOG_FILE_NAME);
    if file_path.exists() {
        return Ok(());
    }

    std::fs::File::create(&file_path)
        .map_err(|e| AppError::FileOperation(format!("创建启动日志文件失败：{}", e)))?;
    Ok(())
}

fn unpack_builtin_skill_packages(builtin_dir: &Path) -> Result<(), AppError> {
    for skill in BUILTIN_SKILLS {
        let skill_dir = builtin_dir.join(skill.dir_name);
        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| AppError::FileOperation(format!("创建内置技能目录失败：{}", e)))?;

        let skill_md = format!(
            "---\nname: {}\ndescription: {}\nlicense: AGPL-3.0\nmetadata:\n  version: {}\n  type: builtin\n  runtime: rust\n  builtin_name: {}\n  display_name: {}\nallowed-tools: file:read file:write\n---\n\n# {}\n\n该技能为系统内置能力的文件化插件包。\n\n- 运行时实现：Rust 内置工具分发\n- 包用途：可视化、审计、离线备份\n",
            skill.dir_name,
            skill.description,
            skill.version,
            skill.name,
            skill.display_name,
            skill.display_name
        );
        std::fs::write(skill_dir.join("SKILL.md"), skill_md)
            .map_err(|e| AppError::FileOperation(format!("写入内置技能 SKILL.md 失败：{}", e)))?;

        let package_json = serde_json::json!({
            "type": "builtin",
            "name": skill.name,
            "display_name": skill.display_name,
            "version": skill.version,
            "runtime": "rust",
            "entry": "builtin-dispatch"
        });
        let package_content = serde_json::to_vec_pretty(&package_json)
            .map_err(|e| AppError::Config(format!("序列化内置技能包描述失败：{}", e)))?;
        std::fs::write(skill_dir.join("package.json"), package_content).map_err(|e| {
            AppError::FileOperation(format!("写入内置技能 package.json 失败：{}", e))
        })?;

        let references_dir = skill_dir.join("references");
        std::fs::create_dir_all(&references_dir)
            .map_err(|e| AppError::FileOperation(format!("创建内置技能引用目录失败：{}", e)))?;
        std::fs::write(
            references_dir.join("README.md"),
            "此目录用于放置该内置技能的参考资料。\n",
        )
        .map_err(|e| AppError::FileOperation(format!("写入内置技能引用说明失败：{}", e)))?;
    }

    Ok(())
}

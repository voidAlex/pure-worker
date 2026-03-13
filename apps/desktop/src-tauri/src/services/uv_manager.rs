//! uv 环境管理服务模块
//!
//! 提供 uv 可用性检测、技能虚拟环境创建、依赖安装与安装修复能力。

use serde::{Deserialize, Serialize};
use specta::Type;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

use crate::error::AppError;

/// uv 子进程通用超时时间（秒）。
const UV_COMMAND_TIMEOUT_SECS: u64 = 120;

/// uv 健康探测超时时间（秒）。
const UV_PROBE_TIMEOUT_SECS: u64 = 10;

/// uv 健康检查结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UvHealthResult {
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub message: String,
}

/// uv 安装或修复执行结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UvInstallResult {
    pub success: bool,
    pub version: Option<String>,
    pub output: String,
}

/// uv 管理服务。
pub struct UvManager;

impl UvManager {
    /// 检查 uv 健康状态，按 PATH 与候选路径回退。
    pub async fn check_uv_health() -> Result<UvHealthResult, AppError> {
        let mut candidates = vec![String::from("uv")];

        if cfg!(windows) {
            if let Ok(profile) = env::var("USERPROFILE") {
                candidates.push(format!(r"{}\.local\bin\uv.exe", profile));
            }
        } else if let Ok(home) = env::var("HOME") {
            candidates.push(format!("{home}/.local/bin/uv"));
        }

        if let Ok(cargo_home) = env::var("CARGO_HOME") {
            let executable = if cfg!(windows) { "uv.exe" } else { "uv" };
            candidates.push(format!("{cargo_home}/bin/{executable}"));
        }

        for candidate in candidates {
            if let Some((version, path)) = Self::probe_uv_candidate(&candidate).await? {
                return Ok(UvHealthResult {
                    available: true,
                    version: Some(version),
                    path: Some(path),
                    message: String::from("uv 可用"),
                });
            }
        }

        Ok(UvHealthResult {
            available: false,
            version: None,
            path: None,
            message: String::from("未检测到 uv，请先安装或修复"),
        })
    }

    /// 创建技能 Python 虚拟环境。
    pub async fn create_skill_env(
        skill_name: &str,
        python_version: Option<&str>,
    ) -> Result<String, AppError> {
        if skill_name.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("skill_name 不能为空")));
        }

        Self::validate_skill_name(skill_name)?;

        let base = Self::skill_env_base_dir()?;
        let env_path = base.join(skill_name);
        tokio::fs::create_dir_all(&base)
            .await
            .map_err(|error| AppError::FileOperation(error.to_string()))?;

        let mut cmd = Command::new("uv");
        cmd.arg("venv").arg(&env_path);
        if let Some(version) = python_version {
            if !version.trim().is_empty() {
                cmd.arg("--python").arg(version.trim());
            }
        }

        let output = run_command_with_timeout(&mut cmd, UV_COMMAND_TIMEOUT_SECS, "uv venv").await?;
        if !output.status.success() {
            return Err(AppError::ExternalService(format!(
                "创建技能环境失败：{}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(env_path.to_string_lossy().to_string())
    }

    /// 在技能环境中安装依赖。
    pub async fn install_skill_deps(
        env_path: &str,
        requirements_path: &str,
    ) -> Result<(), AppError> {
        if env_path.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("env_path 不能为空")));
        }
        if requirements_path.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from(
                "requirements_path 不能为空",
            )));
        }

        let python_path = if cfg!(windows) {
            Path::new(env_path).join("Scripts").join("python.exe")
        } else {
            Path::new(env_path).join("bin").join("python")
        };

        let mut cmd = Command::new("uv");
        cmd.arg("pip")
            .arg("install")
            .arg("-r")
            .arg(requirements_path)
            .arg("--python")
            .arg(&python_path);
        let output =
            run_command_with_timeout(&mut cmd, UV_COMMAND_TIMEOUT_SECS, "uv pip install").await?;

        if !output.status.success() {
            return Err(AppError::ExternalService(format!(
                "安装依赖失败：{}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    /// 安装 uv（仅允许 astral.sh 来源）。
    pub async fn install_uv() -> Result<UvInstallResult, AppError> {
        Self::run_uv_installer().await
    }

    /// 修复 uv（仅允许 astral.sh 来源）。
    pub async fn repair_uv() -> Result<UvInstallResult, AppError> {
        Self::run_uv_installer().await
    }

    /// 校验技能名称，防止目录穿越攻击。
    ///
    /// 仅允许 `[A-Za-z0-9._-]`，禁止路径分隔符和 `..`。
    fn validate_skill_name(skill_name: &str) -> Result<(), AppError> {
        if skill_name.contains("..") || skill_name.contains('/') || skill_name.contains('\\') {
            return Err(AppError::InvalidInput(format!(
                "技能名称包含非法字符（禁止路径分隔符和 '..'）：'{skill_name}'"
            )));
        }

        let valid = skill_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
        if !valid {
            return Err(AppError::InvalidInput(format!(
                "技能名称仅允许字母、数字、点、下划线和连字符：'{skill_name}'"
            )));
        }

        Ok(())
    }

    /// 构建技能环境根目录。
    fn skill_env_base_dir() -> Result<PathBuf, AppError> {
        let home = if cfg!(windows) {
            env::var("USERPROFILE")
        } else {
            env::var("HOME")
        }
        .map_err(|_| AppError::Config(String::from("未找到用户主目录环境变量")))?;

        Ok(Path::new(&home).join(".pureworker").join("skill-envs"))
    }

    /// 探测单个 uv 候选路径。
    async fn probe_uv_candidate(candidate: &str) -> Result<Option<(String, String)>, AppError> {
        let timeout = Duration::from_secs(UV_PROBE_TIMEOUT_SECS);
        let child = match Command::new(candidate)
            .arg("--version")
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(o)) => o,
            _ => return Ok(None),
        };
        if !output.status.success() {
            return Ok(None);
        }

        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if raw.is_empty() {
            return Ok(None);
        }

        let version = raw
            .split_whitespace()
            .nth(1)
            .map(ToOwned::to_owned)
            .or_else(|| Some(raw.clone()));

        let path = if candidate == "uv" {
            String::from("PATH:uv")
        } else {
            candidate.to_string()
        };

        Ok(version.map(|value| (value, path)))
    }

    /// 执行 uv 安装脚本。
    async fn run_uv_installer() -> Result<UvInstallResult, AppError> {
        let output = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("powershell");
            cmd.arg("-NoProfile")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg("irm https://astral.sh/uv/install.ps1 | iex");
            run_command_with_timeout(&mut cmd, UV_COMMAND_TIMEOUT_SECS, "uv 安装脚本").await?
        } else {
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg("curl -LsSf https://astral.sh/uv/install.sh | sh");
            run_command_with_timeout(&mut cmd, UV_COMMAND_TIMEOUT_SECS, "uv 安装脚本").await?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let text_output = if stderr.trim().is_empty() {
            stdout
        } else if stdout.trim().is_empty() {
            stderr
        } else {
            format!("{stdout}\n{stderr}")
        };

        let health = Self::check_uv_health().await?;
        Ok(UvInstallResult {
            success: output.status.success() && health.available,
            version: health.version,
            output: text_output,
        })
    }
}

/// 带超时和 kill_on_drop 防护执行子进程命令。
///
/// 超时后子进程会被自动终止（kill_on_drop），防止僵尸进程。
async fn run_command_with_timeout(
    cmd: &mut Command,
    timeout_secs: u64,
    label: &str,
) -> Result<std::process::Output, AppError> {
    let child = cmd
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::ExternalService(format!("启动 {label} 失败：{e}")))?;

    let timeout = Duration::from_secs(timeout_secs);
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(result) => {
            result.map_err(|e| AppError::ExternalService(format!("等待 {label} 完成失败：{e}")))
        }
        Err(_) => Err(AppError::ExternalService(format!(
            "{label} 执行超时（{timeout_secs} 秒）"
        ))),
    }
}

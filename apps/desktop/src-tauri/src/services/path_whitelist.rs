//! 路径白名单校验模块
//!
//! 对所有文件 I/O 操作进行路径安全检查，确保只允许访问白名单内的目录。
//! 技术方案要求："禁止写入白名单外路径"。
//!
//! 白名单规则：
//! - 用户主目录下的 `.pureworker/` 目录（模型、缓存等）
//! - 系统临时目录（`std::env::temp_dir()`）
//! - 用户文档目录（如 ~/Documents、~/Desktop、~/Downloads）
//!
//! 写入操作额外限制：不允许写入用户主目录根级文件。

use std::path::{Component, PathBuf};

use crate::error::AppError;

/// 路径白名单校验服务。
pub struct PathWhitelistService;

impl PathWhitelistService {
    /// 校验读取路径是否在白名单内。
    ///
    /// 读取操作允许的路径范围比写入更宽松：
    /// - 用户主目录及其所有子目录
    /// - 系统临时目录
    /// - `.pureworker/` 目录
    pub fn validate_read_path(path: &str) -> Result<(), AppError> {
        let canonical = Self::resolve_path(path)?;
        let allowed = Self::get_read_whitelist();

        for dir in &allowed {
            if canonical.starts_with(dir) {
                return Ok(());
            }
        }

        Err(AppError::PermissionDenied(format!(
            "路径不在读取白名单内：'{path}'。允许的目录：{}",
            allowed
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("、")
        )))
    }

    /// 校验写入路径是否在白名单内。
    ///
    /// 写入操作更严格，仅允许以下目录：
    /// - `~/.pureworker/` 目录
    /// - 系统临时目录
    /// - 用户文档/桌面/下载目录
    pub fn validate_write_path(path: &str) -> Result<(), AppError> {
        let canonical = Self::resolve_path(path)?;
        let allowed = Self::get_write_whitelist();

        for dir in &allowed {
            if canonical.starts_with(dir) {
                return Ok(());
            }
        }

        Err(AppError::PermissionDenied(format!(
            "路径不在写入白名单内：'{path}'。允许的目录：{}",
            allowed
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("、")
        )))
    }

    /// 解析并规范化路径（展开 `~`、转绝对路径、词法消除 `.`/`..`）。
    ///
    /// 安全策略：
    /// 1. 展开 `~` 为用户主目录
    /// 2. 相对路径转绝对路径
    /// 3. **词法规范化**：消除所有 `.` 和 `..` 组件，防止路径穿越攻击
    ///    （例如 `~/Documents/nonexistent/../../evil.txt` → `~/evil.txt`）
    /// 4. 若文件存在，额外使用 `canonicalize()` 解析符号链接
    fn resolve_path(path: &str) -> Result<PathBuf, AppError> {
        // 第一步：展开 ~ 前缀
        let expanded = if let Some(stripped) = path.strip_prefix('~') {
            let home = Self::get_home_dir()
                .ok_or_else(|| AppError::FileOperation("无法获取用户主目录".to_string()))?;
            home.join(stripped.strip_prefix('/').unwrap_or(stripped))
        } else {
            PathBuf::from(path)
        };

        // 第二步：转绝对路径（相对路径基于当前工作目录）
        let absolute = std::path::absolute(&expanded)
            .map_err(|e| AppError::FileOperation(format!("路径绝对化失败 '{path}'：{e}")))?;

        // 第三步：词法规范化 —— 消除所有 . 和 .. 组件
        let normalized = Self::normalize_lexical(&absolute);

        // 第四步：若文件/目录已存在，使用 canonicalize 解析符号链接
        if normalized.exists() {
            normalized
                .canonicalize()
                .map_err(|e| AppError::FileOperation(format!("路径规范化失败 '{path}'：{e}")))
        } else if let Some(parent) = normalized.parent() {
            // 文件不存在但父目录存在时，canonicalize 父目录 + join 文件名
            if parent.exists() {
                let canonical_parent = parent.canonicalize().map_err(|e| {
                    AppError::FileOperation(format!("父目录规范化失败 '{}'：{e}", parent.display()))
                })?;
                if let Some(file_name) = normalized.file_name() {
                    return Ok(canonical_parent.join(file_name));
                }
            }
            // 父目录也不存在：使用词法规范化结果（已消除 .. 和 .）
            Ok(normalized)
        } else {
            Ok(normalized)
        }
    }

    /// 词法级路径规范化：消除 `.` 和 `..` 组件，不依赖文件系统。
    ///
    /// 与 `canonicalize()` 不同，此函数不要求路径存在，也不解析符号链接。
    /// 它仅通过分析路径组件来消除穿越序列，确保白名单检查不被绕过。
    ///
    /// 示例：
    /// - `/home/user/Documents/../.secret` → `/home/user/.secret`
    /// - `/home/user/./test.txt` → `/home/user/test.txt`
    fn normalize_lexical(path: &std::path::Path) -> PathBuf {
        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {
                    // `.` 组件直接跳过
                }
                Component::ParentDir => {
                    // `..` 组件：弹出上一级（不能退到根以上）
                    result.pop();
                }
                // RootDir、Prefix、Normal 原样保留
                other => {
                    result.push(other);
                }
            }
        }
        result
    }

    /// 获取读取操作的白名单目录列表。
    fn get_read_whitelist() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // 用户主目录（读取允许全部子目录）
        if let Some(home) = Self::get_home_dir() {
            dirs.push(home);
        }

        // 系统临时目录
        dirs.push(std::env::temp_dir());

        // .pureworker 目录（可能不在 home 下）
        if let Some(home) = Self::get_home_dir() {
            let pureworker = home.join(".pureworker");
            if !dirs.iter().any(|d| pureworker.starts_with(d)) {
                dirs.push(pureworker);
            }
        }

        dirs
    }

    /// 获取写入操作的白名单目录列表。
    fn get_write_whitelist() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // 系统临时目录
        dirs.push(std::env::temp_dir());

        if let Some(home) = Self::get_home_dir() {
            // .pureworker 工作目录
            dirs.push(home.join(".pureworker"));

            // 常用用户目录
            for sub in &["Documents", "Desktop", "Downloads", "文档", "桌面", "下载"] {
                let dir = home.join(sub);
                if dir.exists() {
                    dirs.push(dir);
                }
            }
        }

        dirs
    }

    /// 校验工作区路径是否在安全范围内。
    ///
    /// 工作区路径由前端 IPC 传入，需防止任意路径穿越。
    /// 允许范围：用户主目录子目录、系统临时目录。
    pub fn validate_workspace_path(path: &str) -> Result<(), AppError> {
        let canonical = Self::resolve_path(path)?;
        let mut allowed = Vec::new();

        if let Some(home) = Self::get_home_dir() {
            allowed.push(home);
        }
        allowed.push(std::env::temp_dir());

        for dir in &allowed {
            if canonical.starts_with(dir) {
                return Ok(());
            }
        }

        Err(AppError::PermissionDenied(format!(
            "工作区路径不在安全范围内：'{path}'。仅允许用户主目录或临时目录下的路径"
        )))
    }

    /// 获取用户主目录。
    fn get_home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from)
    }
}

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

use std::path::PathBuf;

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

    /// 解析并规范化路径（展开 ~ 和相对路径）。
    fn resolve_path(path: &str) -> Result<PathBuf, AppError> {
        let expanded = if let Some(stripped) = path.strip_prefix('~') {
            let home = Self::get_home_dir()
                .ok_or_else(|| AppError::FileOperation("无法获取用户主目录".to_string()))?;
            home.join(stripped.strip_prefix('/').unwrap_or(stripped))
        } else {
            PathBuf::from(path)
        };

        // 使用 dunce::canonicalize 或直接用父目录判断
        // 注意：文件可能尚不存在（写入场景），因此先尝试规范化，
        // 失败则使用绝对化后的路径
        if expanded.exists() {
            expanded
                .canonicalize()
                .map_err(|e| AppError::FileOperation(format!("路径规范化失败 '{path}'：{e}")))
        } else {
            // 文件不存在时，尝试规范化父目录
            if let Some(parent) = expanded.parent() {
                if parent.exists() {
                    let canonical_parent = parent.canonicalize().map_err(|e| {
                        AppError::FileOperation(format!(
                            "父目录规范化失败 '{}'：{e}",
                            parent.display()
                        ))
                    })?;
                    if let Some(file_name) = expanded.file_name() {
                        return Ok(canonical_parent.join(file_name));
                    }
                }
            }
            // 最后兜底：使用绝对路径
            Ok(std::path::absolute(&expanded).unwrap_or(expanded))
        }
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

    /// 获取用户主目录。
    fn get_home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from)
    }
}

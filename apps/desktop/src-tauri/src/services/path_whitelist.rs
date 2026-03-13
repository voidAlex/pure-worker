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

use std::path::{Component, Path, PathBuf};

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

    /// 解析并规范化路径（展开 `~`、转绝对路径、词法消除 `.`/`..`、解析符号链接）。
    ///
    /// 安全策略：
    /// 1. 展开 `~` 为用户主目录
    /// 2. 相对路径转绝对路径
    /// 3. **词法规范化**：消除所有 `.` 和 `..` 组件，防止路径穿越攻击
    /// 4. **祖先 canonicalize**：向上查找最近的已存在祖先目录并 canonicalize，
    ///    解析符号链接后重建路径，防止 symlink 逃逸绕过白名单
    fn resolve_path(path: &str) -> Result<PathBuf, AppError> {
        let expanded = if let Some(stripped) = path.strip_prefix('~') {
            let home = Self::get_home_dir()
                .ok_or_else(|| AppError::FileOperation("无法获取用户主目录".to_string()))?;
            home.join(stripped.strip_prefix('/').unwrap_or(stripped))
        } else {
            PathBuf::from(path)
        };

        let absolute = std::path::absolute(&expanded)
            .map_err(|e| AppError::FileOperation(format!("路径绝对化失败 '{path}'：{e}")))?;

        let normalized = Self::normalize_lexical(&absolute);

        // 若完整路径存在，直接 canonicalize（解析所有符号链接）
        if normalized.exists() {
            return normalized
                .canonicalize()
                .map_err(|e| AppError::FileOperation(format!("路径规范化失败 '{path}'：{e}")));
        }

        // 路径不存在时：向上查找最近的已存在祖先目录，canonicalize 后重建
        // 这样能解析路径中任何已存在的 symlink，防止 symlink 逃逸
        Self::canonicalize_nearest_ancestor(&normalized, path)
    }

    /// 向上查找最近的已存在祖先目录，canonicalize 后拼接剩余路径组件。
    ///
    /// 防止 symlink 逃逸攻击：如 `~/Documents/link -> /etc`，
    /// 传入 `~/Documents/link/newdir/file.txt` 时，会发现 `~/Documents/link`
    /// 已存在，canonicalize 为 `/etc`，重建为 `/etc/newdir/file.txt`，
    /// 白名单检查即可正确拒绝。
    fn canonicalize_nearest_ancestor(
        normalized: &std::path::Path,
        original_path: &str,
    ) -> Result<PathBuf, AppError> {
        let mut ancestor = normalized.to_path_buf();
        let mut tail_components: Vec<std::ffi::OsString> = Vec::new();

        // 逐级向上查找，收集不存在的尾部组件
        loop {
            if ancestor.exists() {
                let canonical_ancestor = ancestor.canonicalize().map_err(|e| {
                    AppError::FileOperation(format!(
                        "祖先目录规范化失败 '{}'：{e}",
                        ancestor.display()
                    ))
                })?;
                // 按原始顺序（从祖先到叶子）重建路径
                let mut result = canonical_ancestor;
                for component in tail_components.iter().rev() {
                    result.push(component);
                }
                return Ok(result);
            }

            // 取出最后一个组件加入尾部列表，继续向上
            match ancestor.file_name() {
                Some(name) => {
                    tail_components.push(name.to_os_string());
                    if !ancestor.pop() {
                        break;
                    }
                }
                None => break,
            }
        }

        // 无法找到任何已存在的祖先（极端情况，如根目录不存在）
        // Fail-closed：返回词法规范化结果（已消除 .. 和 .）
        Err(AppError::FileOperation(format!(
            "无法找到路径的任何已存在祖先目录：'{original_path}'"
        )))
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

    /// 校验 workspace 下的 `.agents/skills` 目录安全性，防止 symlink 逃逸。
    ///
    /// 检查 `.agents` 和 `.agents/skills` 是否为符号链接（若存在），
    /// 以及 canonicalize 后的 skills_dir 是否仍在 canonical_workspace 内。
    /// 返回安全的 (canonical_workspace, skills_dir) 路径对。
    ///
    /// 此方法用于 skill_store（写入）和 skill_discovery（读取）场景。
    pub fn validate_skills_dir(workspace_path: &Path) -> Result<(PathBuf, PathBuf), AppError> {
        // workspace 必须存在且为目录
        if !workspace_path.is_dir() {
            return Err(AppError::InvalidInput(format!(
                "工作区路径不存在或不是目录：'{}'",
                workspace_path.display()
            )));
        }

        let canonical_workspace = workspace_path.canonicalize().map_err(|e| {
            AppError::FileOperation(format!(
                "工作区路径规范化失败 '{}'：{e}",
                workspace_path.display()
            ))
        })?;

        let agents_dir = canonical_workspace.join(".agents");
        let skills_dir = agents_dir.join("skills");

        // 若 .agents 已存在，检查是否为 symlink
        if agents_dir.exists() || agents_dir.symlink_metadata().is_ok() {
            let meta = agents_dir.symlink_metadata().map_err(|e| {
                AppError::FileOperation(format!(
                    "无法读取 .agents 目录元数据 '{}'：{e}",
                    agents_dir.display()
                ))
            })?;
            if meta.file_type().is_symlink() {
                return Err(AppError::PermissionDenied(format!(
                    "工作区 .agents 目录是符号链接，已拒绝访问：'{}'",
                    agents_dir.display()
                )));
            }
        }

        // 若 .agents/skills 已存在，检查是否为 symlink
        if skills_dir.exists() || skills_dir.symlink_metadata().is_ok() {
            let meta = skills_dir.symlink_metadata().map_err(|e| {
                AppError::FileOperation(format!(
                    "无法读取 skills 目录元数据 '{}'：{e}",
                    skills_dir.display()
                ))
            })?;
            if meta.file_type().is_symlink() {
                return Err(AppError::PermissionDenied(format!(
                    "工作区 .agents/skills 目录是符号链接，已拒绝访问：'{}'",
                    skills_dir.display()
                )));
            }
        }

        // 若 skills_dir 已存在，二次确认 canonicalize 后仍在 workspace 内
        if skills_dir.exists() {
            let canonical_skills = skills_dir.canonicalize().map_err(|e| {
                AppError::FileOperation(format!(
                    "skills 目录规范化失败 '{}'：{e}",
                    skills_dir.display()
                ))
            })?;
            if !canonical_skills.starts_with(&canonical_workspace) {
                return Err(AppError::PermissionDenied(format!(
                    "skills 目录规范化后逃逸出工作区：'{}'",
                    canonical_skills.display()
                )));
            }
        }

        Ok((canonical_workspace, skills_dir))
    }

    /// 逐级安全创建 workspace 下的 `.agents/skills` 目录。
    ///
    /// 与 `create_dir_all` 不同，此方法在每一级创建后立即进行 symlink 检测，
    /// 防止 TOCTOU 攻击（在 validate_skills_dir 与 create_dir_all 之间
    /// 攻击者将 `.agents` 替换为 symlink 导致越权写入）。
    ///
    /// 流程：
    /// 1. canonicalize workspace → `canonical_workspace`
    /// 2. 对 `.agents`：不存在则 `create_dir`，然后 `symlink_metadata` 确认非 symlink
    /// 3. 对 `.agents/skills`：同上
    /// 4. 最终 canonicalize skills_dir 确认仍在 workspace 边界内
    pub fn ensure_safe_skills_dir(workspace_path: &Path) -> Result<(PathBuf, PathBuf), AppError> {
        if !workspace_path.is_dir() {
            return Err(AppError::InvalidInput(format!(
                "工作区路径不存在或不是目录：'{}'",
                workspace_path.display()
            )));
        }

        let canonical_workspace = workspace_path.canonicalize().map_err(|e| {
            AppError::FileOperation(format!(
                "工作区路径规范化失败 '{}'：{e}",
                workspace_path.display()
            ))
        })?;

        let agents_dir = canonical_workspace.join(".agents");

        // 逐级创建并校验 .agents
        Self::ensure_safe_dir_level(&agents_dir, ".agents")?;

        let skills_dir = agents_dir.join("skills");

        // 逐级创建并校验 .agents/skills
        Self::ensure_safe_dir_level(&skills_dir, ".agents/skills")?;

        // 最终 canonicalize 确认仍在 workspace 边界内
        let canonical_skills = skills_dir.canonicalize().map_err(|e| {
            AppError::FileOperation(format!(
                "skills 目录规范化失败 '{}'：{e}",
                skills_dir.display()
            ))
        })?;
        if !canonical_skills.starts_with(&canonical_workspace) {
            return Err(AppError::PermissionDenied(format!(
                "skills 目录规范化后逃逸出工作区：'{}'",
                canonical_skills.display()
            )));
        }

        Ok((canonical_workspace, skills_dir))
    }

    /// 单级目录安全创建：不存在则 create_dir，创建后立即 symlink_metadata 检测。
    fn ensure_safe_dir_level(dir: &Path, label: &str) -> Result<(), AppError> {
        if !dir.exists() {
            std::fs::create_dir(dir)
                .map_err(|e| AppError::FileOperation(format!("创建 {label} 目录失败：{e}")))?;
        }

        let meta = dir.symlink_metadata().map_err(|e| {
            AppError::FileOperation(format!(
                "无法读取 {label} 目录元数据 '{}'：{e}",
                dir.display()
            ))
        })?;
        if meta.file_type().is_symlink() {
            return Err(AppError::PermissionDenied(format!(
                "工作区 {label} 目录是符号链接，已拒绝访问：'{}'",
                dir.display()
            )));
        }
        if !meta.is_dir() {
            return Err(AppError::InvalidInput(format!(
                "工作区 {label} 不是目录：'{}'",
                dir.display()
            )));
        }

        Ok(())
    }
}

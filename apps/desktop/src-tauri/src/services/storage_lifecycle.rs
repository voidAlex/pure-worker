//! 存储生命周期管理服务模块。
//!
//! 提供工作区导出、归档、擦除与存储统计能力。

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::storage_lifecycle::StorageStats;
use crate::services::app_settings::AppSettingsService;
use crate::services::audit::AuditService;

/// 存储生命周期管理服务。
pub struct StorageLifecycleService;

impl StorageLifecycleService {
    /// 获取工作区目录路径（从 app_settings 读取）。
    pub async fn get_workspace_path(pool: &SqlitePool) -> Result<PathBuf, AppError> {
        let setting = AppSettingsService::get_setting(pool, "workspace_path").await?;
        let value = serde_json::from_str::<String>(&setting.value)
            .unwrap_or_else(|_| setting.value.clone());
        if value.trim().is_empty() {
            return Err(AppError::Config(String::from("workspace_path 配置为空")));
        }

        Ok(PathBuf::from(value))
    }

    /// 导出工作区数据为 ZIP 压缩包。
    pub async fn export_workspace(
        pool: &SqlitePool,
        output_path: &str,
    ) -> Result<String, AppError> {
        if output_path.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("导出路径不能为空")));
        }

        let workspace_path = Self::get_workspace_path(pool).await?;
        if !workspace_path.exists() || !workspace_path.is_dir() {
            return Err(AppError::NotFound(format!(
                "工作区目录不存在：{}",
                workspace_path.display()
            )));
        }

        let output = PathBuf::from(output_path);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| AppError::FileOperation(format!("创建导出目录失败：{error}")))?;
        }

        let staging_root = std::env::temp_dir().join(format!(
            "pure-worker-staging-{}",
            Utc::now().format("%Y%m%d%H%M%S%3f")
        ));
        if staging_root.exists() {
            fs::remove_dir_all(&staging_root).map_err(|error| {
                AppError::FileOperation(format!("清理历史暂存目录失败：{error}"))
            })?;
        }
        fs::create_dir_all(&staging_root)
            .map_err(|error| AppError::FileOperation(format!("创建暂存目录失败：{error}")))?;

        let copy_result = Self::copy_workspace_to_staging(&workspace_path, &staging_root);
        if let Err(copy_error) = copy_result {
            let _ = fs::remove_dir_all(&staging_root);
            return Err(copy_error);
        }

        let zip_result = Self::zip_directory(&staging_root, &output);
        let _ = fs::remove_dir_all(&staging_root);
        zip_result?;

        AuditService::log(
            pool,
            "system",
            "export_workspace",
            "workspace",
            None,
            "high",
            true,
        )
        .await?;

        Ok(output.to_string_lossy().to_string())
    }

    /// 归档工作区数据（移动到归档目录并清空工作区）。
    pub async fn archive_workspace(
        pool: &SqlitePool,
        archive_name: &str,
    ) -> Result<String, AppError> {
        if archive_name.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("归档名称不能为空")));
        }

        let workspace_path = Self::get_workspace_path(pool).await?;
        if !workspace_path.exists() || !workspace_path.is_dir() {
            return Err(AppError::NotFound(format!(
                "工作区目录不存在：{}",
                workspace_path.display()
            )));
        }

        let archives_dir = workspace_path.join("archives");
        fs::create_dir_all(&archives_dir)
            .map_err(|error| AppError::FileOperation(format!("创建归档目录失败：{error}")))?;

        let archive_path = archives_dir.join(archive_name);
        if archive_path.exists() {
            return Err(AppError::InvalidInput(format!(
                "归档已存在，请更换名称：{}",
                archive_path.display()
            )));
        }
        fs::create_dir_all(&archive_path)
            .map_err(|error| AppError::FileOperation(format!("创建归档目标目录失败：{error}")))?;

        for entry in fs::read_dir(&workspace_path)
            .map_err(|error| AppError::FileOperation(format!("读取工作区失败：{error}")))?
        {
            let entry = entry
                .map_err(|error| AppError::FileOperation(format!("读取目录项失败：{error}")))?;
            let path = entry.path();
            if path == archives_dir {
                continue;
            }

            let target = archive_path.join(entry.file_name());
            fs::rename(&path, &target)
                .map_err(|error| AppError::FileOperation(format!("移动归档文件失败：{error}")))?;
        }

        AuditService::log(
            pool,
            "system",
            "archive_workspace",
            "workspace",
            None,
            "high",
            true,
        )
        .await?;

        Ok(archive_path.to_string_lossy().to_string())
    }

    /// 安全擦除工作区数据（覆写后删除）。
    pub async fn erase_workspace(pool: &SqlitePool) -> Result<(), AppError> {
        let workspace_path = Self::get_workspace_path(pool).await?;
        if !workspace_path.exists() || !workspace_path.is_dir() {
            return Err(AppError::NotFound(format!(
                "工作区目录不存在：{}",
                workspace_path.display()
            )));
        }

        for entry in fs::read_dir(&workspace_path)
            .map_err(|error| AppError::FileOperation(format!("读取工作区失败：{error}")))?
        {
            let entry = entry
                .map_err(|error| AppError::FileOperation(format!("读取目录项失败：{error}")))?;
            let path = entry.path();
            Self::secure_remove_path(&path)?;
        }

        AuditService::log(
            pool,
            "system",
            "erase_workspace",
            "workspace",
            None,
            "critical",
            true,
        )
        .await?;

        Ok(())
    }

    /// 获取工作区存储统计信息。
    pub async fn get_storage_stats(pool: &SqlitePool) -> Result<StorageStats, AppError> {
        let workspace_path = Self::get_workspace_path(pool).await?;
        let mut total_files = 0_u64;
        let mut total_size_bytes = 0_u64;

        if workspace_path.exists() && workspace_path.is_dir() {
            Self::collect_stats(&workspace_path, &mut total_files, &mut total_size_bytes)?;
        }

        let archives_dir = workspace_path.join("archives");
        let archive_count = if archives_dir.exists() && archives_dir.is_dir() {
            let mut count = 0_u64;
            for entry in fs::read_dir(&archives_dir)
                .map_err(|error| AppError::FileOperation(format!("读取归档目录失败：{error}")))?
            {
                let entry = entry
                    .map_err(|error| AppError::FileOperation(format!("读取归档项失败：{error}")))?;
                if entry
                    .file_type()
                    .map_err(|error| AppError::FileOperation(format!("读取归档类型失败：{error}")))?
                    .is_dir()
                {
                    count += 1;
                }
            }
            count
        } else {
            0
        };

        Ok(StorageStats {
            workspace_path: workspace_path.to_string_lossy().to_string(),
            total_files,
            total_size_bytes,
            total_size_display: Self::format_size(total_size_bytes),
            archive_count,
        })
    }

    /// 递归统计目录文件数和总大小。
    fn collect_stats(
        path: &Path,
        total_files: &mut u64,
        total_size_bytes: &mut u64,
    ) -> Result<(), AppError> {
        for entry in fs::read_dir(path)
            .map_err(|error| AppError::FileOperation(format!("读取目录失败：{error}")))?
        {
            let entry = entry
                .map_err(|error| AppError::FileOperation(format!("读取目录项失败：{error}")))?;
            let entry_path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| AppError::FileOperation(format!("读取文件类型失败：{error}")))?;

            if file_type.is_dir() {
                Self::collect_stats(&entry_path, total_files, total_size_bytes)?;
            } else if file_type.is_file() {
                *total_files += 1;
                let metadata = fs::metadata(&entry_path).map_err(|error| {
                    AppError::FileOperation(format!("读取文件元信息失败：{error}"))
                })?;
                *total_size_bytes += metadata.len();
            }
        }

        Ok(())
    }

    /// 将工作区内容复制到临时暂存目录（导出前快照）。
    fn copy_workspace_to_staging(workspace: &Path, staging: &Path) -> Result<(), AppError> {
        for entry in fs::read_dir(workspace)
            .map_err(|error| AppError::FileOperation(format!("读取工作区失败：{error}")))?
        {
            let entry = entry.map_err(|error| {
                AppError::FileOperation(format!("读取工作区目录项失败：{error}"))
            })?;
            let source_path = entry.path();
            let file_name = entry.file_name();
            let target_path = staging.join(file_name);

            if source_path.is_dir() {
                Self::copy_dir_recursive(&source_path, &target_path)?;
            } else if source_path.is_file() {
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        AppError::FileOperation(format!("创建暂存目录失败：{error}"))
                    })?;
                }
                fs::copy(&source_path, &target_path).map_err(|error| {
                    AppError::FileOperation(format!("复制文件到暂存目录失败：{error}"))
                })?;
            }
        }

        Ok(())
    }

    /// 递归复制目录。
    fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), AppError> {
        fs::create_dir_all(target)
            .map_err(|error| AppError::FileOperation(format!("创建目标目录失败：{error}")))?;

        for entry in fs::read_dir(source)
            .map_err(|error| AppError::FileOperation(format!("读取源目录失败：{error}")))?
        {
            let entry = entry
                .map_err(|error| AppError::FileOperation(format!("读取源目录项失败：{error}")))?;
            let source_path = entry.path();
            let target_path = target.join(entry.file_name());

            if source_path.is_dir() {
                Self::copy_dir_recursive(&source_path, &target_path)?;
            } else if source_path.is_file() {
                fs::copy(&source_path, &target_path).map_err(|error| {
                    AppError::FileOperation(format!("复制目录文件失败：{error}"))
                })?;
            }
        }

        Ok(())
    }

    /// 将目录压缩为 ZIP 文件。
    fn zip_directory(source_dir: &Path, output_file: &Path) -> Result<(), AppError> {
        let file = fs::File::create(output_file)
            .map_err(|error| AppError::FileOperation(format!("创建 ZIP 文件失败：{error}")))?;
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        Self::zip_walk(source_dir, source_dir, &mut zip, options)?;

        zip.finish()
            .map_err(|error| AppError::FileOperation(format!("完成 ZIP 写入失败：{error}")))?;
        Ok(())
    }

    /// 递归写入 ZIP 条目。
    fn zip_walk(
        base: &Path,
        current: &Path,
        zip: &mut zip::ZipWriter<fs::File>,
        options: zip::write::SimpleFileOptions,
    ) -> Result<(), AppError> {
        for entry in fs::read_dir(current)
            .map_err(|error| AppError::FileOperation(format!("读取目录失败：{error}")))?
        {
            let entry = entry
                .map_err(|error| AppError::FileOperation(format!("读取目录项失败：{error}")))?;
            let path = entry.path();
            let rel = path.strip_prefix(base).map_err(|error| {
                AppError::FileOperation(format!("计算压缩相对路径失败：{error}"))
            })?;

            if path.is_dir() {
                let dir_name = format!("{}/", rel.to_string_lossy());
                zip.add_directory(dir_name, options).map_err(|error| {
                    AppError::FileOperation(format!("写入 ZIP 目录失败：{error}"))
                })?;
                Self::zip_walk(base, &path, zip, options)?;
            } else if path.is_file() {
                let file_name = rel.to_string_lossy().to_string();
                zip.start_file(file_name, options).map_err(|error| {
                    AppError::FileOperation(format!("写入 ZIP 文件头失败：{error}"))
                })?;

                let mut src = fs::File::open(&path).map_err(|error| {
                    AppError::FileOperation(format!("读取待压缩文件失败：{error}"))
                })?;
                let mut buffer = Vec::new();
                src.read_to_end(&mut buffer).map_err(|error| {
                    AppError::FileOperation(format!("读取文件内容失败：{error}"))
                })?;
                zip.write_all(&buffer).map_err(|error| {
                    AppError::FileOperation(format!("写入 ZIP 内容失败：{error}"))
                })?;
            }
        }

        Ok(())
    }

    /// 覆写后删除指定路径（文件或目录）。
    fn secure_remove_path(path: &Path) -> Result<(), AppError> {
        if path.is_dir() {
            for entry in fs::read_dir(path)
                .map_err(|error| AppError::FileOperation(format!("读取目录失败：{error}")))?
            {
                let entry = entry
                    .map_err(|error| AppError::FileOperation(format!("读取目录项失败：{error}")))?;
                Self::secure_remove_path(&entry.path())?;
            }
            fs::remove_dir(path)
                .map_err(|error| AppError::FileOperation(format!("删除目录失败：{error}")))?;
            return Ok(());
        }

        if path.is_file() {
            let metadata = fs::metadata(path)
                .map_err(|error| AppError::FileOperation(format!("读取文件信息失败：{error}")))?;
            let size = metadata.len() as usize;

            if size > 0 {
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .open(path)
                    .map_err(|error| {
                        AppError::FileOperation(format!("打开文件覆写失败：{error}"))
                    })?;
                let zeros = vec![0_u8; size];
                file.write_all(&zeros)
                    .map_err(|error| AppError::FileOperation(format!("覆写文件失败：{error}")))?;
                file.flush()
                    .map_err(|error| AppError::FileOperation(format!("刷新文件失败：{error}")))?;
            }

            fs::remove_file(path)
                .map_err(|error| AppError::FileOperation(format!("删除文件失败：{error}")))?;
        }

        Ok(())
    }

    /// 格式化字节数为可读字符串。
    fn format_size(bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;

        let value = bytes as f64;
        if value >= GB {
            format!("{:.2} GB", value / GB)
        } else if value >= MB {
            format!("{:.2} MB", value / MB)
        } else if value >= KB {
            format!("{:.2} KB", value / KB)
        } else {
            format!("{} B", bytes)
        }
    }

    /// 生成默认归档名称（时间戳）。
    pub fn default_archive_name() -> String {
        format!("archive-{}", Utc::now().format("%Y%m%d%H%M%S"))
    }
}

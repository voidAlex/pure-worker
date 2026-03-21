//! 数据库模块 - PureWorker
//!
//! 提供 SQLite 连接池配置：
//! - WAL 日志模式，支持并发读写
//! - Synchronous NORMAL，平衡性能与安全
//! - 5000ms 忙等待超时，高并发支持
//! - 强制外键约束，每个连接生效
//! - 启动时自动执行数据库迁移

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;
use std::time::Duration;
use tauri::Manager;

use crate::error::AppError;
use crate::services::runtime_paths;

/// Database file name
const DB_NAME: &str = "pureworker.db";

fn get_database_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let workspace_path = runtime_paths::resolve_workspace_path(app_handle)?;
    runtime_paths::ensure_workspace_layout(&workspace_path)?;
    let target_path = runtime_paths::database_file_path(&workspace_path);

    migrate_legacy_database_if_needed(app_handle, &target_path)?;
    Ok(target_path)
}

fn migrate_legacy_database_if_needed(
    app_handle: &tauri::AppHandle,
    target_db_path: &std::path::Path,
) -> Result<(), AppError> {
    if target_db_path.exists() {
        return Ok(());
    }

    let legacy_db_path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("无法获取应用数据目录：{}", e)))?
        .join(DB_NAME);

    if !legacy_db_path.exists() {
        return Ok(());
    }

    if let Some(parent) = target_db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Internal(format!("无法创建数据库目录：{}", e)))?;
    }

    std::fs::copy(&legacy_db_path, target_db_path)
        .map_err(|e| AppError::Internal(format!("迁移数据库文件失败：{}", e)))?;
    copy_sidecar_if_exists(&legacy_db_path, target_db_path, "-wal")?;
    copy_sidecar_if_exists(&legacy_db_path, target_db_path, "-shm")
}

fn copy_sidecar_if_exists(
    source_db_path: &std::path::Path,
    target_db_path: &std::path::Path,
    suffix: &str,
) -> Result<(), AppError> {
    let source = PathBuf::from(format!("{}{}", source_db_path.display(), suffix));
    if !source.exists() {
        return Ok(());
    }

    let target = PathBuf::from(format!("{}{}", target_db_path.display(), suffix));
    std::fs::copy(&source, &target)
        .map_err(|e| AppError::Internal(format!("迁移数据库附属文件失败：{}", e)))?;
    Ok(())
}

/// Initialize SQLite connection pool with PRAGMA settings and run migrations
///
/// # Arguments
/// * `app_handle` - Tauri application handle for getting app data directory
///
/// # Returns
/// * `SqlitePool` - Configured connection pool
///
/// # Errors
/// Returns `AppError::Database` if:
/// - Database connection fails
/// - PRAGMA settings fail
/// - Migrations fail
pub async fn init_pool(app_handle: &tauri::AppHandle) -> Result<SqlitePool, AppError> {
    let db_path = get_database_path(app_handle)?;

    println!("[Database] Initializing database at: {:?}", db_path);

    // Configure connection options with PRAGMA settings
    // These are applied on each new connection
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_millis(5000))
        .foreign_keys(true);

    // Create connection pool with options
    // The pool will use the configured options for each connection
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Database(format!("数据库连接失败：{}", e)))?;

    println!("[Database] Connection pool created successfully");

    // Run migrations
    run_migrations(&pool).await?;

    println!("[Database] Migrations completed successfully");

    Ok(pool)
}

/// 执行数据库迁移
///
/// 使用 sqlx::migrate!() 宏在编译期嵌入迁移文件，
/// 确保打包后的二进制文件不依赖外部 migrations 目录
async fn run_migrations(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Database(format!("执行迁移失败：{}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test database path generation
    #[test]
    fn test_db_name_constant() {
        assert_eq!(DB_NAME, "pureworker.db");
    }
}

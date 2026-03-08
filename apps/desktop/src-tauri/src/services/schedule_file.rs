//! 课表文件服务模块
//!
//! 提供课表事件关联文件的管理功能，包括文件注册、列表查询等

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::schedule_file::{CreateScheduleFileInput, ScheduleFile};

/// 课表文件服务，负责管理课表事件关联的教案/课件文件
pub struct ScheduleFileService;

impl ScheduleFileService {
    /// 注册一个新文件
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `input`: 创建文件输入参数
    pub async fn register(
        pool: &SqlitePool,
        input: CreateScheduleFileInput,
    ) -> Result<ScheduleFile, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO schedule_file (id, class_id, file_name, file_path, file_type, file_size, is_deleted, created_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?)",
        )
        .bind(&id)
        .bind(&input.class_id)
        .bind(&input.file_name)
        .bind(&input.file_path)
        .bind(&input.file_type)
        .bind(input.file_size)
        .bind(&now)
        .execute(pool)
        .await?;

        Self::get_by_id(pool, &id).await
    }

    /// 根据ID获取文件
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<ScheduleFile, AppError> {
        let file = sqlx::query_as::<_, ScheduleFile>(
            "SELECT id, class_id, file_name, file_path, file_type, file_size, is_deleted, created_at FROM schedule_file WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("文件不存在：{id}")))?;

        Ok(file)
    }

    /// 获取班级的所有文件列表
    pub async fn list_by_class(
        pool: &SqlitePool,
        class_id: &str,
    ) -> Result<Vec<ScheduleFile>, AppError> {
        let files = sqlx::query_as::<_, ScheduleFile>(
            "SELECT id, class_id, file_name, file_path, file_type, file_size, is_deleted, created_at FROM schedule_file WHERE class_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(class_id)
        .fetch_all(pool)
        .await?;

        Ok(files)
    }

    /// 删除文件（软删除）
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let result =
            sqlx::query("UPDATE schedule_file SET is_deleted = 1 WHERE id = ? AND is_deleted = 0")
                .bind(id)
                .execute(pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("文件不存在：{id}")));
        }

        Ok(())
    }
}

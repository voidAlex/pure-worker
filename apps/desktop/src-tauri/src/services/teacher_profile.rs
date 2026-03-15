use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::teacher_profile::TeacherProfile;

/// 教师档案服务层
/// 提供教师档案的查询和创建功能
pub struct TeacherProfileService;

impl TeacherProfileService {
    /// 获取或创建默认教师档案
    ///
    /// # 逻辑
    /// 1. 查询数据库中第一个未删除的教师档案
    /// 2. 如果不存在，则创建一个默认档案
    /// 3. 返回教师档案
    pub async fn get_or_create_default(pool: &SqlitePool) -> Result<TeacherProfile, AppError> {
        // 首先尝试查询已存在的教师档案
        let existing = sqlx::query_as::<_, TeacherProfile>(
      "SELECT id, name, stage, subject, textbook_version, tone_preset, is_deleted, created_at, updated_at 
       FROM teacher_profile 
       WHERE is_deleted = 0 
       LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("查询教师档案失败：{}", e)))?;

        if let Some(profile) = existing {
            return Ok(profile);
        }

        // 不存在则创建默认教师档案
        let teacher_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let default_name = "我的教师档案".to_string();
        let default_stage = "".to_string();
        let default_subject = "".to_string();

        sqlx::query(
      "INSERT INTO teacher_profile (id, name, stage, subject, textbook_version, tone_preset, is_deleted, created_at, updated_at) 
       VALUES (?, ?, ?, ?, NULL, NULL, 0, ?, ?)",
    )
    .bind(&teacher_id)
    .bind(&default_name)
    .bind(&default_stage)
    .bind(&default_subject)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("创建默认教师档案失败：{}", e)))?;

        Ok(TeacherProfile {
            id: teacher_id,
            name: default_name,
            stage: default_stage,
            subject: default_subject,
            textbook_version: None,
            tone_preset: None,
            is_deleted: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

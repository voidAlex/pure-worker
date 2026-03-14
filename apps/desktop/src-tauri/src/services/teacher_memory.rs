//! 教师偏好记忆服务模块
//!
//! 提供教师偏好的数据库操作、候选记忆管理、以及模式检测功能。

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::teacher_memory::{
    ConfirmCandidateInput, ListCandidatesInput, MemoryCandidate, RejectCandidateInput,
    SetPreferenceInput, TeacherPreference,
};
use crate::services::audit::AuditService;

/// 教师偏好记忆服务
pub struct TeacherMemoryService;

impl TeacherMemoryService {
    /// 获取单个活跃偏好
    pub async fn get_preference(
        pool: &SqlitePool,
        key: &str,
    ) -> Result<TeacherPreference, AppError> {
        let item = sqlx::query_as::<_, TeacherPreference>(
            "SELECT id, preference_key, preference_value, preference_type, source, confirmed_at, is_active, is_deleted, created_at, updated_at 
             FROM teacher_preference 
             WHERE preference_key = ? AND is_active = 1 AND is_deleted = 0",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("偏好不存在：{}", key)))?;

        Ok(item)
    }

    /// 获取偏好值（如果不存在返回 None）
    pub async fn get_preference_optional(
        pool: &SqlitePool,
        key: &str,
    ) -> Result<Option<TeacherPreference>, AppError> {
        let item = sqlx::query_as::<_, TeacherPreference>(
            "SELECT id, preference_key, preference_value, preference_type, source, confirmed_at, is_active, is_deleted, created_at, updated_at 
             FROM teacher_preference 
             WHERE preference_key = ? AND is_active = 1 AND is_deleted = 0",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?;

        Ok(item)
    }

    /// 列出所有活跃偏好
    pub async fn list_preferences(pool: &SqlitePool) -> Result<Vec<TeacherPreference>, AppError> {
        let items = sqlx::query_as::<_, TeacherPreference>(
            "SELECT id, preference_key, preference_value, preference_type, source, confirmed_at, is_active, is_deleted, created_at, updated_at 
             FROM teacher_preference 
             WHERE is_active = 1 AND is_deleted = 0 
             ORDER BY updated_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 按类型列出偏好
    pub async fn list_preferences_by_type(
        pool: &SqlitePool,
        pref_type: &str,
    ) -> Result<Vec<TeacherPreference>, AppError> {
        let items = sqlx::query_as::<_, TeacherPreference>(
            "SELECT id, preference_key, preference_value, preference_type, source, confirmed_at, is_active, is_deleted, created_at, updated_at 
             FROM teacher_preference 
             WHERE preference_type = ? AND is_active = 1 AND is_deleted = 0 
             ORDER BY updated_at DESC",
        )
        .bind(pref_type)
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 设置偏好（创建或更新）
    pub async fn set_preference(
        pool: &SqlitePool,
        input: &SetPreferenceInput,
    ) -> Result<TeacherPreference, AppError> {
        if input.key.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("key 不能为空")));
        }
        if input.value.trim().is_empty() {
            return Err(AppError::InvalidInput(String::from("value 不能为空")));
        }

        let now = Utc::now().to_rfc3339();
        let source = input
            .source
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "explicit".to_string());

        // 检查是否已存在
        let existing = sqlx::query_as::<_, TeacherPreference>(
            "SELECT id, preference_key, preference_value, preference_type, source, confirmed_at, is_active, is_deleted, created_at, updated_at 
             FROM teacher_preference 
             WHERE preference_key = ? AND is_deleted = 0",
        )
        .bind(&input.key)
        .fetch_optional(pool)
        .await?;

        let pref_type_str = input.preference_type.to_string();
        let target_id = if let Some(item) = existing {
            // 更新现有偏好
            sqlx::query(
                "UPDATE teacher_preference 
                 SET preference_value = ?, preference_type = ?, source = ?, confirmed_at = ?, updated_at = ? 
                 WHERE id = ? AND is_deleted = 0",
            )
            .bind(&input.value)
            .bind(&pref_type_str)
            .bind(&source)
            .bind(&now)
            .bind(&now)
            .bind(&item.id)
            .execute(pool)
            .await?;
            item.id
        } else {
            // 创建新偏好
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO teacher_preference 
                 (id, preference_key, preference_value, preference_type, source, confirmed_at, is_active, is_deleted, created_at, updated_at) 
                 VALUES (?, ?, ?, ?, ?, ?, 1, 0, ?, ?)",
            )
            .bind(&id)
            .bind(&input.key)
            .bind(&input.value)
            .bind(&pref_type_str)
            .bind(&source)
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
            id
        };

        AuditService::log(
            pool,
            "system",
            "set_teacher_preference",
            "teacher_preference",
            Some(&target_id),
            "medium",
            false,
        )
        .await?;

        Self::get_preference(pool, &input.key).await
    }

    /// 删除偏好（软删除）
    pub async fn delete_preference(pool: &SqlitePool, key: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE teacher_preference 
             SET is_deleted = 1, updated_at = ? 
             WHERE preference_key = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(key)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("偏好不存在：{}", key)));
        }

        AuditService::log(
            pool,
            "system",
            "delete_teacher_preference",
            "teacher_preference",
            Some(key),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 列出候选记忆
    pub async fn list_candidates(
        pool: &SqlitePool,
        input: &ListCandidatesInput,
    ) -> Result<Vec<MemoryCandidate>, AppError> {
        let mut query = String::from(
            "SELECT id, candidate_key, candidate_value, detected_count, confidence_score, pattern_evidence, status, confirmed_at, rejected_at, rejection_reason, is_deleted, created_at, updated_at 
             FROM memory_candidate 
             WHERE is_deleted = 0",
        );

        if let Some(_status) = &input.status {
            query.push_str(" AND status = ?");
        }

        query.push_str(" ORDER BY detected_count DESC, created_at DESC");

        if let Some(limit) = input.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let items = if let Some(status) = &input.status {
            sqlx::query_as::<_, MemoryCandidate>(&query)
                .bind(status.to_string())
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query_as::<_, MemoryCandidate>(&query)
                .fetch_all(pool)
                .await?
        };

        Ok(items)
    }

    /// 确认候选记忆
    pub async fn confirm_candidate(
        pool: &SqlitePool,
        input: &ConfirmCandidateInput,
    ) -> Result<TeacherPreference, AppError> {
        let now = Utc::now().to_rfc3339();

        // 获取候选记忆
        let candidate: MemoryCandidate = sqlx::query_as(
            "SELECT id, candidate_key, candidate_value, detected_count, confidence_score, pattern_evidence, status, confirmed_at, rejected_at, rejection_reason, is_deleted, created_at, updated_at 
             FROM memory_candidate 
             WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.candidate_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("候选记忆不存在：{}", input.candidate_id))
        })?;

        if candidate.status != "pending" {
            return Err(AppError::InvalidInput(format!(
                "候选记忆状态不是 pending：{}",
                candidate.status
            )));
        }

        // 更新候选记忆状态
        sqlx::query(
            "UPDATE memory_candidate 
             SET status = 'confirmed', confirmed_at = ?, updated_at = ? 
             WHERE id = ?",
        )
        .bind(&now)
        .bind(&now)
        .bind(&input.candidate_id)
        .execute(pool)
        .await?;

        // 创建正式偏好记录
        let pref_input = SetPreferenceInput {
            key: candidate.candidate_key,
            value: candidate.candidate_value,
            preference_type: crate::models::teacher_memory::PreferenceType::Other,
            source: Some(crate::models::teacher_memory::PreferenceSource::Inferred),
        };

        let preference = Self::set_preference(pool, &pref_input).await?;

        AuditService::log(
            pool,
            "system",
            "confirm_memory_candidate",
            "memory_candidate",
            Some(&input.candidate_id),
            "medium",
            false,
        )
        .await?;

        Ok(preference)
    }

    /// 拒绝候选记忆
    pub async fn reject_candidate(
        pool: &SqlitePool,
        input: &RejectCandidateInput,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();

        let candidate: MemoryCandidate = sqlx::query_as(
            "SELECT id, candidate_key, candidate_value, detected_count, confidence_score, pattern_evidence, status, confirmed_at, rejected_at, rejection_reason, is_deleted, created_at, updated_at 
             FROM memory_candidate 
             WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.candidate_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("候选记忆不存在：{}", input.candidate_id))
        })?;

        if candidate.status != "pending" {
            return Err(AppError::InvalidInput(format!(
                "候选记忆状态不是 pending：{}",
                candidate.status
            )));
        }

        sqlx::query(
            "UPDATE memory_candidate 
             SET status = 'rejected', rejected_at = ?, rejection_reason = ?, updated_at = ? 
             WHERE id = ?",
        )
        .bind(&now)
        .bind(&input.reason)
        .bind(&now)
        .bind(&input.candidate_id)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "reject_memory_candidate",
            "memory_candidate",
            Some(&input.candidate_id),
            "medium",
            false,
        )
        .await?;

        Ok(())
    }

    /// 记录模式检测
    pub async fn record_pattern_detection(
        pool: &SqlitePool,
        pattern_type: &str,
        pattern_key: &str,
        pattern_value: Option<&str>,
        context_hash: Option<&str>,
    ) -> Result<bool, AppError> {
        let now = Utc::now().to_rfc3339();

        // 检查是否已有记录
        let existing = sqlx::query_as::<_, (i32, String)>(
            "SELECT occurrence_count, id FROM preference_detection_pattern 
             WHERE pattern_type = ? AND pattern_key = ? AND is_deleted = 0",
        )
        .bind(pattern_type)
        .bind(pattern_key)
        .fetch_optional(pool)
        .await?;

        let should_create_candidate = if let Some((count, id)) = existing {
            let new_count = count + 1;
            sqlx::query(
                "UPDATE preference_detection_pattern 
                 SET occurrence_count = ?, last_occurred_at = ?, updated_at = ? 
                 WHERE id = ?",
            )
            .bind(new_count)
            .bind(&now)
            .bind(&now)
            .bind(&id)
            .execute(pool)
            .await?;

            new_count >= 3
        } else {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO preference_detection_pattern 
                 (id, pattern_type, pattern_key, pattern_value, occurrence_count, last_occurred_at, context_hash, is_deleted, created_at, updated_at) 
                 VALUES (?, ?, ?, ?, 1, ?, ?, 0, ?, ?)",
            )
            .bind(&id)
            .bind(pattern_type)
            .bind(pattern_key)
            .bind(pattern_value)
            .bind(&now)
            .bind(context_hash)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;

            false
        };

        // 如果检测次数达到 3 次，创建候选记忆
        if should_create_candidate {
            // 检查是否已存在相同的待处理候选
            let existing_candidate = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM memory_candidate 
                 WHERE candidate_key = ? AND status = 'pending' AND is_deleted = 0",
            )
            .bind(format!("{}.{}", pattern_type, pattern_key))
            .fetch_one(pool)
            .await?;

            if existing_candidate == 0 {
                let candidate_id = Uuid::new_v4().to_string();
                let candidate_key = format!("{}.{}", pattern_type, pattern_key);
                let candidate_value = pattern_value.unwrap_or("detected_pattern").to_string();

                sqlx::query(
                    "INSERT INTO memory_candidate 
                     (id, candidate_key, candidate_value, detected_count, confidence_score, pattern_evidence, status, is_deleted, created_at, updated_at) 
                     VALUES (?, ?, ?, 3, 0.8, ?, 'pending', 0, ?, ?)",
                )
                .bind(&candidate_id)
                .bind(&candidate_key)
                .bind(&candidate_value)
                .bind(format!("Pattern detected: {} = {}", pattern_key, candidate_value))
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
        }

        Ok(should_create_candidate)
    }

    /// 构建系统提示词上下文
    pub async fn build_system_prompt_context(
        pool: &SqlitePool,
    ) -> Result<crate::models::teacher_memory::SystemPromptContext, AppError> {
        let preferences = Self::list_preferences(pool).await?;

        // 按优先级排序：explicit > inferred > imported > default
        let mut formatted = String::from("## 教师偏好设定\n\n");

        for pref in &preferences {
            let priority_marker = match pref.source.as_str() {
                "explicit" => "【显式设置】",
                "inferred" => "【推断偏好】",
                "imported" => "【导入设置】",
                _ => "【默认设置】",
            };
            formatted.push_str(&format!(
                "- {} {}: {}\n",
                priority_marker, pref.preference_key, pref.preference_value
            ));
        }

        Ok(crate::models::teacher_memory::SystemPromptContext {
            soul_md_content: None,
            user_md_content: None,
            active_preferences: preferences,
            formatted_context: formatted,
        })
    }
}

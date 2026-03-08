use chrono::Utc;
use sqlx::QueryBuilder;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::observation_note::ObservationNote;
use crate::models::parent_communication::ParentCommunication;
use crate::models::score_record::ScoreRecord;
use crate::models::student::{CreateStudentInput, Student, StudentProfile360, UpdateStudentInput};
use crate::models::student_tag::StudentTag;
use crate::services::audit::AuditService;

pub struct StudentService;

impl StudentService {
    pub async fn list(pool: &SqlitePool, class_id: Option<&str>) -> Result<Vec<Student>, AppError> {
        let mut query = QueryBuilder::new(
            "SELECT id, student_no, name, gender, class_id, meta_json, folder_path, is_deleted, created_at, updated_at FROM student WHERE is_deleted = 0",
        );

        if let Some(class_id) = class_id {
            query.push(" AND class_id = ").push_bind(class_id);
        }

        query.push(" ORDER BY created_at DESC");

        let students = query.build_query_as::<Student>().fetch_all(pool).await?;
        Ok(students)
    }

    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Student, AppError> {
        let student = sqlx::query_as::<_, Student>(
            "SELECT id, student_no, name, gender, class_id, meta_json, folder_path, is_deleted, created_at, updated_at FROM student WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("学生不存在：{id}")))?;

        Ok(student)
    }

    pub async fn create(pool: &SqlitePool, input: CreateStudentInput) -> Result<Student, AppError> {
        Self::validate_classroom_exists(pool, &input.class_id).await?;
        Self::validate_unique_student_no(pool, &input.student_no, &input.class_id, None).await?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let folder_path = format!("workspace/students/{id}/");

        sqlx::query(
            "INSERT INTO student (id, student_no, name, gender, class_id, meta_json, folder_path, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.student_no)
        .bind(&input.name)
        .bind(&input.gender)
        .bind(&input.class_id)
        .bind(&input.meta_json)
        .bind(&folder_path)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        AuditService::log(
            pool,
            "system",
            "create_student",
            "student",
            Some(&id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &id).await
    }

    pub async fn update(pool: &SqlitePool, input: UpdateStudentInput) -> Result<Student, AppError> {
        let has_updates = input.student_no.is_some()
            || input.name.is_some()
            || input.gender.is_some()
            || input.class_id.is_some()
            || input.meta_json.is_some();

        if !has_updates {
            return Err(AppError::InvalidInput(String::from(
                "至少提供一个需要更新的字段",
            )));
        }

        let existing = Self::get_by_id(pool, &input.id).await?;

        let target_class_id = input
            .class_id
            .clone()
            .unwrap_or_else(|| existing.class_id.clone());
        let target_student_no = input
            .student_no
            .clone()
            .unwrap_or_else(|| existing.student_no.clone());

        if target_class_id != existing.class_id {
            Self::validate_classroom_exists(pool, &target_class_id).await?;
        }

        if target_class_id != existing.class_id || target_student_no != existing.student_no {
            Self::validate_unique_student_no(
                pool,
                &target_student_no,
                &target_class_id,
                Some(&input.id),
            )
            .await?;
        }

        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE student SET student_no = COALESCE(?, student_no), name = COALESCE(?, name), gender = COALESCE(?, gender), class_id = COALESCE(?, class_id), meta_json = COALESCE(?, meta_json), updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.student_no)
        .bind(&input.name)
        .bind(&input.gender)
        .bind(&input.class_id)
        .bind(&input.meta_json)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("学生不存在：{}", input.id)));
        }

        AuditService::log(
            pool,
            "system",
            "update_student",
            "student",
            Some(&input.id),
            "medium",
            false,
        )
        .await?;

        Self::get_by_id(pool, &input.id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE student SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("学生不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_student",
            "student",
            Some(id),
            "high",
            false,
        )
        .await?;

        Ok(())
    }

    /// 获取学生 360 度全景视图（聚合标签、成绩、观察、沟通）
    pub async fn get_profile_360(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<StudentProfile360, AppError> {
        let student = Self::get_by_id(pool, id).await?;

        let tags = sqlx::query_as::<_, StudentTag>(
            "SELECT id, student_id, tag_name, is_deleted, created_at FROM student_tag WHERE student_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        let recent_scores = sqlx::query_as::<_, ScoreRecord>(
            "SELECT id, student_id, exam_name, subject, score, full_score, rank_in_class, exam_date, is_deleted, updated_at FROM score_record WHERE student_id = ? AND is_deleted = 0 ORDER BY exam_date DESC LIMIT 10",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        let recent_observations = sqlx::query_as::<_, ObservationNote>(
            "SELECT id, student_id, content, source, created_at, is_deleted, updated_at FROM observation_note WHERE student_id = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT 10",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        let recent_communications = sqlx::query_as::<_, ParentCommunication>(
            "SELECT id, student_id, draft, adopted_text, status, evidence_json, created_at, is_deleted, updated_at FROM parent_communication WHERE student_id = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT 10",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        Ok(StudentProfile360 {
            student,
            tags,
            recent_scores,
            recent_observations,
            recent_communications,
        })
    }

    async fn validate_classroom_exists(pool: &SqlitePool, class_id: &str) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM classroom WHERE id = ? AND is_deleted = 0",
        )
        .bind(class_id)
        .fetch_one(pool)
        .await?;

        if exists == 0 {
            return Err(AppError::InvalidInput(format!(
                "班级不存在或已删除：{class_id}"
            )));
        }

        Ok(())
    }

    async fn validate_unique_student_no(
        pool: &SqlitePool,
        student_no: &str,
        class_id: &str,
        exclude_id: Option<&str>,
    ) -> Result<(), AppError> {
        let exists = if let Some(exclude_id) = exclude_id {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM student WHERE student_no = ? AND class_id = ? AND is_deleted = 0 AND id <> ?",
            )
            .bind(student_no)
            .bind(class_id)
            .bind(exclude_id)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM student WHERE student_no = ? AND class_id = ? AND is_deleted = 0",
            )
            .bind(student_no)
            .bind(class_id)
            .fetch_one(pool)
            .await?
        };

        if exists > 0 {
            return Err(AppError::InvalidInput(format!(
                "同一班级下学号已存在：student_no={student_no}, class_id={class_id}"
            )));
        }

        Ok(())
    }
}

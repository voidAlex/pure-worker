//! 作业批改服务模块
//! 负责 M4 作业批改场景下的核心 CRUD 能力：批改任务、作业素材、OCR 结果、错题记录、题库与导出。

use std::path::{Path, PathBuf};

use chrono::Utc;
use rust_xlsxwriter::Workbook;
use sqlx::{QueryBuilder, SqlitePool};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::assignment_grading::{
    AddAssignmentAssetsInput, AssignmentAsset, AssignmentOcrResult, BatchReviewOcrResultsInput,
    CreateGradingJobInput, CreateQuestionBankInput, ExportGradingResultsInput,
    ExportGradingResultsResponse, GradingJob, ListQuestionBankInput, ListWrongAnswersInput,
    QuestionBankItem, ReviewOcrResultInput, UpdateGradingJobInput, WrongAnswerRecord,
};
use crate::services::audit::AuditService;

/// 作业批改服务。
pub struct AssignmentGradingService;

impl AssignmentGradingService {
    /// 规范化文件路径，统一为绝对路径并清理 `.`、`..` 片段。
    fn normalize_file_path(file_path: &str) -> Result<PathBuf, AppError> {
        let raw_path = Path::new(file_path);
        let absolute_path = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            let current_dir = std::env::current_dir()
                .map_err(|e| AppError::FileOperation(format!("获取当前目录失败: {e}")))?;
            current_dir.join(raw_path)
        };

        let mut normalized = PathBuf::new();
        for component in absolute_path.components() {
            match component {
                std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                std::path::Component::RootDir => normalized.push(component.as_os_str()),
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::Normal(part) => normalized.push(part),
            }
        }

        Ok(std::fs::canonicalize(&normalized).unwrap_or(normalized))
    }

    /// 校验文件路径是否落在白名单目录内。
    fn validate_file_path_whitelist(
        file_path: &str,
        workspace_path: Option<&str>,
    ) -> Result<(), AppError> {
        let normalized_file_path = Self::normalize_file_path(file_path)?;

        let mut allowed_dirs: Vec<PathBuf> = Vec::new();
        let home_dir = std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
            .or_else(|| {
                let home_drive = std::env::var("HOMEDRIVE").ok();
                let home_path = std::env::var("HOMEPATH").ok();
                match (home_drive, home_path) {
                    (Some(drive), Some(path)) => Some(PathBuf::from(format!("{drive}{path}"))),
                    _ => None,
                }
            });

        if let Some(home) = home_dir {
            let normalized_home = Self::normalize_file_path(home.to_string_lossy().as_ref())?;
            allowed_dirs.push(normalized_home.clone());
            allowed_dirs.push(normalized_home.join("Documents"));
            allowed_dirs.push(normalized_home.join("Desktop"));
            allowed_dirs.push(normalized_home.join("Downloads"));
            allowed_dirs.push(normalized_home.join("Pictures"));
        }

        if let Some(workspace) = workspace_path.filter(|path| !path.trim().is_empty()) {
            allowed_dirs.push(Self::normalize_file_path(workspace)?);
        }

        let in_whitelist = allowed_dirs
            .iter()
            .any(|allowed_dir| normalized_file_path.starts_with(allowed_dir));

        if in_whitelist {
            return Ok(());
        }

        Err(AppError::PermissionDenied(format!(
            "文件路径不在允许访问范围内: {}",
            normalized_file_path.display()
        )))
    }

    /// 从 OCR 文本中提取学号（6-12 位数字）。
    fn extract_student_no_from_text(text: &str) -> Option<String> {
        let marker = "学号";
        let start = text.find(marker)? + marker.len();
        let mut seen_digits = false;
        let mut digits = String::new();

        for ch in text[start..].chars() {
            if !seen_digits {
                if ch == '：' || ch == ':' || ch.is_whitespace() {
                    continue;
                }
                if ch.is_ascii_digit() {
                    seen_digits = true;
                    digits.push(ch);
                    continue;
                }
                break;
            }

            if ch.is_ascii_digit() {
                digits.push(ch);
            } else {
                break;
            }
        }

        if (6..=12).contains(&digits.len()) {
            Some(digits)
        } else {
            None
        }
    }

    /// 从 OCR 文本中提取姓名（2-4 位中文字符）。
    fn extract_student_name_from_text(text: &str) -> Option<String> {
        let marker = "姓名";
        let start = text.find(marker)? + marker.len();
        let mut seen_name = false;
        let mut name = String::new();

        for ch in text[start..].chars() {
            if !seen_name {
                if ch == '：' || ch == ':' || ch.is_whitespace() {
                    continue;
                }
                if ('\u{4e00}'..='\u{9fa5}').contains(&ch) {
                    seen_name = true;
                    name.push(ch);
                    continue;
                }
                break;
            }

            if ('\u{4e00}'..='\u{9fa5}').contains(&ch) {
                if name.chars().count() >= 4 {
                    break;
                }
                name.push(ch);
            } else {
                break;
            }
        }

        if (2..=4).contains(&name.chars().count()) {
            Some(name)
        } else {
            None
        }
    }

    /// 创建批改任务。
    pub async fn create_grading_job(
        pool: &SqlitePool,
        input: CreateGradingJobInput,
    ) -> Result<GradingJob, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO grading_job (id, class_id, title, grading_mode, status, answer_key_json, scoring_rules_json, total_assets, processed_assets, failed_assets, conflict_count, task_id, output_path, is_deleted, created_at, updated_at) \
             VALUES (?, ?, ?, COALESCE(?, 'basic'), 'pending', ?, ?, 0, 0, 0, 0, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.class_id)
        .bind(&input.title)
        .bind(&input.grading_mode)
        .bind(&input.answer_key_json)
        .bind(&input.scoring_rules_json)
        .bind(&input.task_id)
        .bind(&input.output_path)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("创建批改任务失败: {e}")))?;

        AuditService::log(
            pool,
            "system",
            "create_grading_job",
            "grading_job",
            Some(&id),
            "low",
            false,
        )
        .await?;

        Self::get_grading_job(pool, &id).await
    }

    /// 获取单个批改任务。
    pub async fn get_grading_job(pool: &SqlitePool, id: &str) -> Result<GradingJob, AppError> {
        sqlx::query_as::<_, GradingJob>("SELECT * FROM grading_job WHERE id = ? AND is_deleted = 0")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(format!("查询批改任务失败: {e}")))?
            .ok_or_else(|| AppError::NotFound("未找到批改任务".to_string()))
    }

    /// 列出班级下的批改任务。
    pub async fn list_grading_jobs(
        pool: &SqlitePool,
        class_id: &str,
    ) -> Result<Vec<GradingJob>, AppError> {
        sqlx::query_as::<_, GradingJob>(
            "SELECT * FROM grading_job WHERE class_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(class_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询批改任务列表失败: {e}")))
    }

    /// 更新批改任务。
    pub async fn update_grading_job(
        pool: &SqlitePool,
        input: UpdateGradingJobInput,
    ) -> Result<GradingJob, AppError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE grading_job SET \
             title = COALESCE(?, title), \
             grading_mode = COALESCE(?, grading_mode), \
             status = COALESCE(?, status), \
             answer_key_json = COALESCE(?, answer_key_json), \
             scoring_rules_json = COALESCE(?, scoring_rules_json), \
             task_id = COALESCE(?, task_id), \
             output_path = COALESCE(?, output_path), \
             updated_at = ? \
             WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.title)
        .bind(&input.grading_mode)
        .bind(&input.status)
        .bind(&input.answer_key_json)
        .bind(&input.scoring_rules_json)
        .bind(&input.task_id)
        .bind(&input.output_path)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("更新批改任务失败: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到可更新的批改任务".to_string()));
        }

        AuditService::log(
            pool,
            "system",
            "update_grading_job",
            "grading_job",
            Some(&input.id),
            "low",
            false,
        )
        .await?;

        Self::get_grading_job(pool, &input.id).await
    }

    /// 软删除批改任务。
    pub async fn delete_grading_job(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE grading_job SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("删除批改任务失败: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到可删除的批改任务".to_string()));
        }

        AuditService::log(
            pool,
            "system",
            "delete_grading_job",
            "grading_job",
            Some(id),
            "medium",
            true,
        )
        .await?;
        Ok(())
    }

    /// 批量添加作业素材。
    pub async fn add_assignment_assets(
        pool: &SqlitePool,
        input: AddAssignmentAssetsInput,
    ) -> Result<Vec<AssignmentAsset>, AppError> {
        if input.file_paths.is_empty() {
            return Err(AppError::InvalidInput("文件路径不能为空".to_string()));
        }

        let now = Utc::now().to_rfc3339();
        let mut created_ids = Vec::with_capacity(input.file_paths.len());

        for file_path in &input.file_paths {
            Self::validate_file_path_whitelist(file_path, None)?;

            let metadata = tokio::fs::metadata(file_path)
                .await
                .map_err(|e| AppError::FileOperation(format!("读取文件信息失败: {e}")))?;

            let file_size = i64::try_from(metadata.len())
                .map_err(|_| AppError::InvalidInput("文件大小超出支持范围".to_string()))?;

            let original_filename = Path::new(file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| AppError::InvalidInput("无法解析文件名".to_string()))?
                .to_string();

            let id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO assignment_asset (id, class_id, file_path, job_id, original_filename, file_size, preprocess_status, is_deleted, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, 'pending', 0, ?, ?)",
            )
            .bind(&id)
            .bind(&input.class_id)
            .bind(file_path)
            .bind(&input.job_id)
            .bind(&original_filename)
            .bind(file_size)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("写入作业素材失败: {e}")))?;
            created_ids.push(id);
        }

        sqlx::query(
            "UPDATE grading_job SET total_assets = total_assets + ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(i32::try_from(input.file_paths.len()).map_err(|_| {
            AppError::InvalidInput("素材数量超出支持范围".to_string())
        })?)
        .bind(&now)
        .bind(&input.job_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("更新任务素材数量失败: {e}")))?;

        AuditService::log(
            pool,
            "system",
            "add_assignment_assets",
            "assignment_asset",
            Some(&input.job_id),
            "low",
            false,
        )
        .await?;

        let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT * FROM assignment_asset WHERE is_deleted = 0 AND id IN (",
        );
        {
            let mut separated = qb.separated(", ");
            for id in &created_ids {
                separated.push_bind(id);
            }
        }
        qb.push(") ORDER BY created_at DESC");

        qb.build_query_as::<AssignmentAsset>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("查询新增素材失败: {e}")))
    }

    /// 列出任务下的素材。
    pub async fn list_job_assets(
        pool: &SqlitePool,
        job_id: &str,
    ) -> Result<Vec<AssignmentAsset>, AppError> {
        sqlx::query_as::<_, AssignmentAsset>(
            "SELECT * FROM assignment_asset WHERE job_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询任务素材失败: {e}")))
    }

    /// 删除单个素材（软删除）。
    pub async fn delete_assignment_asset(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE assignment_asset SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("删除素材失败: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到可删除的素材".to_string()));
        }

        AuditService::log(
            pool,
            "system",
            "delete_assignment_asset",
            "assignment_asset",
            Some(id),
            "medium",
            true,
        )
        .await?;
        Ok(())
    }

    /// 列出任务下的 OCR 结果。
    pub async fn list_job_ocr_results(
        pool: &SqlitePool,
        job_id: &str,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        sqlx::query_as::<_, AssignmentOcrResult>(
            "SELECT * FROM assignment_ocr_result WHERE job_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
            .map_err(|e| AppError::Database(format!("查询 OCR 结果失败: {e}")))
    }

    /// 从 OCR 文本中匹配学生信息并回填 student_id。
    pub async fn match_student_from_ocr(pool: &SqlitePool, job_id: &str) -> Result<i32, AppError> {
        let rows = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
            "SELECT id, ocr_raw_text, answer_text \
             FROM assignment_ocr_result \
             WHERE job_id = ? AND student_id IS NULL AND is_deleted = 0",
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询待匹配 OCR 记录失败: {e}")))?;

        let mut matched_count = 0_i32;

        for (ocr_id, ocr_raw_text, answer_text) in rows {
            let mut student_no: Option<String> = None;
            let mut student_name: Option<String> = None;

            for text in [ocr_raw_text.as_deref(), answer_text.as_deref()]
                .into_iter()
                .flatten()
            {
                if student_no.is_none() {
                    student_no = Self::extract_student_no_from_text(text);
                }
                if student_name.is_none() {
                    student_name = Self::extract_student_name_from_text(text);
                }
                if student_no.is_some() || student_name.is_some() {
                    break;
                }
            }

            if student_no.is_none() && student_name.is_none() {
                continue;
            }

            let matched_student_id = sqlx::query_scalar::<_, String>(
                "SELECT id FROM student \
                 WHERE (student_no = ? OR name = ?) AND is_deleted = 0 \
                 ORDER BY created_at ASC \
                 LIMIT 1",
            )
            .bind(student_no.as_deref().unwrap_or(""))
            .bind(student_name.as_deref().unwrap_or(""))
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Database(format!("查询学生匹配失败: {e}")))?;

            if let Some(student_id) = matched_student_id {
                sqlx::query(
                    "UPDATE assignment_ocr_result SET student_id = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&student_id)
                .bind(Utc::now().to_rfc3339())
                .bind(&ocr_id)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(format!("更新 OCR 学生匹配失败: {e}")))?;
                matched_count += 1;
            }
        }

        AuditService::log(
            pool,
            "system",
            "match_student_from_ocr",
            "assignment_ocr_result",
            Some(job_id),
            "low",
            false,
        )
        .await?;

        Ok(matched_count)
    }

    /// 复核单条 OCR 结果。
    pub async fn review_ocr_result(
        pool: &SqlitePool,
        input: ReviewOcrResultInput,
    ) -> Result<AssignmentOcrResult, AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE assignment_ocr_result \
             SET review_status = ?, final_score = ?, reviewed_by = ?, reviewed_at = ?, updated_at = ? \
             WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.review_status)
        .bind(input.final_score)
        .bind(&input.reviewed_by)
        .bind(&now)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("复核 OCR 结果失败: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到可复核的 OCR 结果".to_string()));
        }

        AuditService::log(
            pool,
            "system",
            "review_ocr_result",
            "assignment_ocr_result",
            Some(&input.id),
            "medium",
            true,
        )
        .await?;

        sqlx::query_as::<_, AssignmentOcrResult>(
            "SELECT * FROM assignment_ocr_result WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询复核结果失败: {e}")))?
        .ok_or_else(|| AppError::NotFound("未找到 OCR 结果".to_string()))
    }

    /// 批量复核 OCR 结果。
    pub async fn batch_review_ocr_results(
        pool: &SqlitePool,
        input: BatchReviewOcrResultsInput,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        if input.ids.is_empty() {
            return Err(AppError::InvalidInput("批量复核 ID 不能为空".to_string()));
        }

        let now = Utc::now().to_rfc3339();
        for id in &input.ids {
            sqlx::query(
                "UPDATE assignment_ocr_result \
                 SET review_status = ?, final_score = ?, reviewed_by = ?, reviewed_at = ?, updated_at = ? \
                 WHERE id = ? AND is_deleted = 0",
            )
            .bind(&input.review_status)
            .bind(input.final_score)
            .bind(&input.reviewed_by)
            .bind(&now)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("批量复核 OCR 结果失败: {e}")))?;
        }

        AuditService::log(
            pool,
            "system",
            "batch_review_ocr_results",
            "assignment_ocr_result",
            None,
            "medium",
            true,
        )
        .await?;

        let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT * FROM assignment_ocr_result WHERE is_deleted = 0 AND id IN (",
        );
        {
            let mut separated = qb.separated(", ");
            for id in &input.ids {
                separated.push_bind(id);
            }
        }
        qb.push(") ORDER BY created_at DESC");

        qb.build_query_as::<AssignmentOcrResult>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("查询批量复核结果失败: {e}")))
    }

    /// 查询错题记录（支持动态筛选）。
    pub async fn list_wrong_answers(
        pool: &SqlitePool,
        input: ListWrongAnswersInput,
    ) -> Result<Vec<WrongAnswerRecord>, AppError> {
        let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT * FROM wrong_answer_record WHERE is_deleted = 0 AND created_at >= datetime('now', '-30 days')",
        );

        if let Some(job_id) = &input.job_id {
            qb.push(" AND job_id = ").push_bind(job_id);
        }
        if let Some(student_id) = &input.student_id {
            qb.push(" AND student_id = ").push_bind(student_id);
        }
        if let Some(knowledge_point) = &input.knowledge_point {
            qb.push(" AND knowledge_point = ")
                .push_bind(knowledge_point);
        }
        if input.unresolved_only.unwrap_or(false) {
            qb.push(" AND is_resolved = 0");
        }

        qb.push(" ORDER BY created_at DESC");
        if let Some(limit) = input.limit {
            qb.push(" LIMIT ").push_bind(limit);
        }

        qb.build_query_as::<WrongAnswerRecord>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("查询错题记录失败: {e}")))
    }

    /// 标记错题已解决。
    pub async fn resolve_wrong_answer(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<WrongAnswerRecord, AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE wrong_answer_record SET is_resolved = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("更新错题状态失败: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到可更新的错题记录".to_string()));
        }

        AuditService::log(
            pool,
            "system",
            "resolve_wrong_answer",
            "wrong_answer_record",
            Some(id),
            "low",
            false,
        )
        .await?;

        sqlx::query_as::<_, WrongAnswerRecord>(
            "SELECT * FROM wrong_answer_record WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询错题记录失败: {e}")))?
        .ok_or_else(|| AppError::NotFound("未找到错题记录".to_string()))
    }

    /// 查询题库（支持动态筛选）。
    pub async fn list_question_bank(
        pool: &SqlitePool,
        input: ListQuestionBankInput,
    ) -> Result<Vec<QuestionBankItem>, AppError> {
        let mut qb =
            QueryBuilder::<sqlx::Sqlite>::new("SELECT * FROM question_bank WHERE is_deleted = 0");

        if let Some(source) = &input.source {
            qb.push(" AND source = ").push_bind(source);
        }
        if let Some(knowledge_point) = &input.knowledge_point {
            qb.push(" AND knowledge_point = ")
                .push_bind(knowledge_point);
        }
        if let Some(difficulty) = &input.difficulty {
            qb.push(" AND difficulty = ").push_bind(difficulty);
        }
        if let Some(question_type) = &input.question_type {
            qb.push(" AND question_type = ").push_bind(question_type);
        }
        if let Some(subject) = &input.subject {
            qb.push(" AND subject = ").push_bind(subject);
        }
        if let Some(grade) = &input.grade {
            qb.push(" AND grade = ").push_bind(grade);
        }

        qb.push(" ORDER BY created_at DESC");
        if let Some(limit) = input.limit {
            qb.push(" LIMIT ").push_bind(limit);
        }

        qb.build_query_as::<QuestionBankItem>()
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("查询题库失败: {e}")))
    }

    /// 创建题库条目。
    pub async fn create_question_bank_item(
        pool: &SqlitePool,
        input: CreateQuestionBankInput,
    ) -> Result<QuestionBankItem, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO question_bank (id, source, knowledge_point, difficulty, stem, answer, explanation, question_type, subject, grade, tags_json, template_params_json, parent_id, is_deleted, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.source)
        .bind(&input.knowledge_point)
        .bind(&input.difficulty)
        .bind(&input.stem)
        .bind(&input.answer)
        .bind(&input.explanation)
        .bind(&input.question_type)
        .bind(&input.subject)
        .bind(&input.grade)
        .bind(&input.tags_json)
        .bind(&input.template_params_json)
        .bind(&input.parent_id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("创建题库条目失败: {e}")))?;

        AuditService::log(
            pool,
            "system",
            "create_question_bank_item",
            "question_bank",
            Some(&id),
            "low",
            false,
        )
        .await?;

        sqlx::query_as::<_, QuestionBankItem>(
            "SELECT * FROM question_bank WHERE id = ? AND is_deleted = 0",
        )
        .bind(&id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询题库条目失败: {e}")))?
        .ok_or_else(|| AppError::NotFound("未找到题库条目".to_string()))
    }

    /// 导出批改结果到 Excel。
    pub async fn export_grading_results(
        pool: &SqlitePool,
        input: ExportGradingResultsInput,
    ) -> Result<ExportGradingResultsResponse, AppError> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<f64>, Option<f64>, i32, Option<f64>, String)>(
            "SELECT COALESCE(s.name, ''), COALESCE(s.student_no, ''), COALESCE(aor.question_no, ''), COALESCE(aor.answer_text, ''), aor.score, aor.confidence, aor.conflict_flag, aor.final_score, COALESCE(aor.review_status, 'pending') \
             FROM assignment_ocr_result aor \
             LEFT JOIN student s ON s.id = aor.student_id AND s.is_deleted = 0 \
             WHERE aor.job_id = ? AND aor.is_deleted = 0 \
             ORDER BY s.name ASC, aor.question_no ASC",
        )
        .bind(&input.job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询导出数据失败: {e}")))?;

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        worksheet
            .write_string(0, 0, "学生姓名")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 1, "学号")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 2, "题号")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 3, "答案文本")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 4, "OCR得分")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 5, "置信度")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 6, "冲突标记")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 7, "最终得分")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 8, "复核状态")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;

        for (index, row) in rows.iter().enumerate() {
            let line = u32::try_from(index + 1)
                .map_err(|_| AppError::TaskExecution("导出数据行数超出限制".to_string()))?;
            worksheet
                .write_string(line, 0, &row.0)
                .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            worksheet
                .write_string(line, 1, &row.1)
                .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            worksheet
                .write_string(line, 2, &row.2)
                .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            worksheet
                .write_string(line, 3, &row.3)
                .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            if let Some(score) = row.4 {
                worksheet
                    .write_number(line, 4, score)
                    .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            }
            if let Some(confidence) = row.5 {
                worksheet
                    .write_number(line, 5, confidence)
                    .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            }
            worksheet
                .write_number(line, 6, f64::from(row.6))
                .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            if let Some(final_score) = row.7 {
                worksheet
                    .write_number(line, 7, final_score)
                    .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            }
            worksheet
                .write_string(line, 8, &row.8)
                .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        }

        workbook
            .save(&input.output_path)
            .map_err(|e| AppError::FileOperation(format!("保存导出文件失败: {e}")))?;

        AuditService::log(
            pool,
            "system",
            "export_grading_results",
            "grading_job",
            Some(&input.job_id),
            "high",
            true,
        )
        .await?;

        Ok(ExportGradingResultsResponse {
            output_path: input.output_path,
            total_rows: i32::try_from(rows.len())
                .map_err(|_| AppError::TaskExecution("导出行数超出支持范围".to_string()))?,
        })
    }

    /// 更新任务处理进度。
    pub async fn update_job_progress(
        pool: &SqlitePool,
        job_id: &str,
        processed: i32,
        failed: i32,
        conflicts: i32,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE grading_job \
             SET processed_assets = ?, failed_assets = ?, conflict_count = ?, \
                 status = CASE WHEN (? + ?) >= total_assets THEN 'completed' ELSE status END, \
                 updated_at = ? \
             WHERE id = ? AND is_deleted = 0",
        )
        .bind(processed)
        .bind(failed)
        .bind(conflicts)
        .bind(processed)
        .bind(failed)
        .bind(&now)
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("更新任务进度失败: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到需要更新进度的任务".to_string()));
        }

        AuditService::log(
            pool,
            "system",
            "update_job_progress",
            "grading_job",
            Some(job_id),
            "low",
            false,
        )
        .await?;

        Ok(())
    }

    /// 更新任务状态。
    pub async fn update_job_status(
        pool: &SqlitePool,
        job_id: &str,
        status: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE grading_job SET status = ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(status)
        .bind(&now)
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("更新任务状态失败: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("未找到需要更新状态的任务".to_string()));
        }

        AuditService::log(
            pool,
            "system",
            "update_job_status",
            "grading_job",
            Some(job_id),
            "low",
            false,
        )
        .await?;

        Ok(())
    }
}

//! 作业批改服务模块
//! 负责 M4 作业批改场景下的核心 CRUD 能力：批改任务、作业素材、OCR 结果、错题记录、题库与导出。

use std::path::Path;

use chrono::Utc;
use rust_xlsxwriter::Workbook;
use sqlx::{QueryBuilder, SqlitePool};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::assignment_grading::{
    AddAssignmentAssetsInput, AssignmentAsset, AssignmentOcrResult, BatchReviewOcrResultsInput,
    CreateGradingJobInput, CreateQuestionBankInput, ExportGradingResultsInput,
    ExportGradingResultsResponse, GradingJob, ListQuestionBankInput, ListWrongAnswersInput,
    QuestionBankItem, ReviewOcrResultInput, UpdateGradingJobInput,
    WrongAnswerRecord,
};
use crate::services::audit::AuditService;

/// 作业批改服务。
pub struct AssignmentGradingService;

impl AssignmentGradingService {
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
            "SELECT * FROM wrong_answer_record WHERE is_deleted = 0",
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
        let rows = sqlx::query_as::<_, (String, String, String, Option<f64>, Option<f64>, String)>(
            "SELECT COALESCE(ps.student_name, ''), aor.question_no, COALESCE(aor.answer_text, ''), aor.score, aor.final_score, COALESCE(aor.review_status, 'pending') \
             FROM assignment_ocr_result aor \
             LEFT JOIN practice_sheet ps ON ps.id = aor.student_id AND ps.is_deleted = 0 \
             WHERE aor.job_id = ? AND aor.is_deleted = 0 \
             ORDER BY ps.student_name ASC, aor.question_no ASC",
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
            .write_string(0, 1, "题号")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 2, "答案文本")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 3, "OCR得分")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 4, "最终得分")
            .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
        worksheet
            .write_string(0, 5, "复核状态")
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
            if let Some(score) = row.3 {
                worksheet
                    .write_number(line, 3, score)
                    .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            }
            if let Some(final_score) = row.4 {
                worksheet
                    .write_number(line, 4, final_score)
                    .map_err(|e| AppError::TaskExecution(format!("写入 Excel 失败: {e}")))?;
            }
            worksheet
                .write_string(line, 5, &row.5)
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

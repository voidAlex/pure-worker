//! 作业批改 IPC 命令模块
//!
//! 暴露批改任务管理、作业资产处理、OCR 结果审核、错题管理、
//! 练习卷生成、题库管理等 Tauri IPC 命令。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::{Manager, State};

use crate::error::AppError;
use crate::models::assignment_grading::{
    AddAssignmentAssetsInput, AssignmentAsset, AssignmentOcrResult, BatchReviewOcrResultsInput,
    CreateGradingJobInput, CreateQuestionBankInput, ExportGradingResultsInput,
    ExportGradingResultsResponse, GeneratePracticeSheetInput, GradingJob, ListQuestionBankInput,
    ListWrongAnswersInput, PracticeSheet, QuestionBankItem, ReviewOcrResultInput,
    StartGradingInput, UpdateGradingJobInput, WrongAnswerRecord,
};
use crate::models::async_task::AsyncTask;
use crate::services;
use crate::services::async_task::AsyncTaskService;

/// 获取应用工作区路径。
fn get_workspace_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| AppError::FileOperation(format!("获取数据目录失败：{error}")))?;
    Ok(data_dir.join("workspace"))
}

/// 列出某个班级批改任务的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct ListGradingJobsInput {
    /// 班级 ID。
    pub class_id: String,
}

/// 获取单个批改任务的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct GetGradingJobInput {
    /// 批改任务 ID。
    pub id: String,
}

/// 删除批改任务的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct DeleteGradingJobInput {
    /// 批改任务 ID。
    pub id: String,
}

/// 列出任务下作业资产的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct ListJobAssetsInput {
    /// 批改任务 ID。
    pub job_id: String,
}

/// 删除作业资产的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct DeleteAssetInput {
    /// 作业资产 ID。
    pub id: String,
}

/// 列出任务 OCR 结果的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct ListJobOcrResultsInput {
    /// 批改任务 ID。
    pub job_id: String,
}

/// 列出冲突 OCR 结果的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct ListConflictResultsInput {
    /// 批改任务 ID。
    pub job_id: String,
}

/// 解决错题记录的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct ResolveWrongAnswerCommandInput {
    /// 错题记录 ID。
    pub id: String,
}

/// 获取练习卷详情的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct GetPracticeSheetInput {
    /// 练习卷 ID。
    pub id: String,
}

/// 列出学生练习卷的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct ListStudentPracticeSheetsInput {
    /// 学生 ID。
    pub student_id: String,
}

/// 删除练习卷的请求参数。
#[derive(Debug, Deserialize, Type)]
pub struct DeletePracticeSheetInput {
    /// 练习卷 ID。
    pub id: String,
}

/// 删除类命令的通用响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteResponse {
    /// 删除是否成功。
    pub success: bool,
}

/// 创建批改任务。
#[tauri::command]
#[specta::specta]
pub async fn create_grading_job(
    pool: State<'_, SqlitePool>,
    input: CreateGradingJobInput,
) -> Result<GradingJob, AppError> {
    services::assignment_grading::AssignmentGradingService::create_grading_job(&pool, input).await
}

/// 获取批改任务详情。
#[tauri::command]
#[specta::specta]
pub async fn get_grading_job(
    pool: State<'_, SqlitePool>,
    input: GetGradingJobInput,
) -> Result<GradingJob, AppError> {
    services::assignment_grading::AssignmentGradingService::get_grading_job(&pool, &input.id).await
}

/// 列出班级下所有批改任务。
#[tauri::command]
#[specta::specta]
pub async fn list_grading_jobs(
    pool: State<'_, SqlitePool>,
    input: ListGradingJobsInput,
) -> Result<Vec<GradingJob>, AppError> {
    services::assignment_grading::AssignmentGradingService::list_grading_jobs(
        &pool,
        &input.class_id,
    )
    .await
}

/// 更新批改任务。
#[tauri::command]
#[specta::specta]
pub async fn update_grading_job(
    pool: State<'_, SqlitePool>,
    input: UpdateGradingJobInput,
) -> Result<GradingJob, AppError> {
    services::assignment_grading::AssignmentGradingService::update_grading_job(&pool, input).await
}

/// 删除批改任务。
#[tauri::command]
#[specta::specta]
pub async fn delete_grading_job(
    pool: State<'_, SqlitePool>,
    input: DeleteGradingJobInput,
) -> Result<DeleteResponse, AppError> {
    services::assignment_grading::AssignmentGradingService::delete_grading_job(&pool, &input.id)
        .await?;
    Ok(DeleteResponse { success: true })
}

/// 批量添加作业资产。
#[tauri::command]
#[specta::specta]
pub async fn add_assignment_assets(
    pool: State<'_, SqlitePool>,
    input: AddAssignmentAssetsInput,
) -> Result<Vec<AssignmentAsset>, AppError> {
    services::assignment_grading::AssignmentGradingService::add_assignment_assets(&pool, input)
        .await
}

/// 列出批改任务下的作业资产。
#[tauri::command]
#[specta::specta]
pub async fn list_job_assets(
    pool: State<'_, SqlitePool>,
    input: ListJobAssetsInput,
) -> Result<Vec<AssignmentAsset>, AppError> {
    services::assignment_grading::AssignmentGradingService::list_job_assets(&pool, &input.job_id)
        .await
}

/// 删除作业资产。
#[tauri::command]
#[specta::specta]
pub async fn delete_assignment_asset(
    pool: State<'_, SqlitePool>,
    input: DeleteAssetInput,
) -> Result<DeleteResponse, AppError> {
    services::assignment_grading::AssignmentGradingService::delete_assignment_asset(
        &pool, &input.id,
    )
    .await?;
    Ok(DeleteResponse { success: true })
}

/// 启动作业批改异步任务。
#[tauri::command]
#[specta::specta]
pub async fn start_grading(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: StartGradingInput,
) -> Result<AsyncTask, AppError> {
    let job = services::assignment_grading::AssignmentGradingService::get_grading_job(
        &pool,
        &input.job_id,
    )
    .await?;
    let assets = services::assignment_grading::AssignmentGradingService::list_job_assets(
        &pool,
        &input.job_id,
    )
    .await?;

    services::assignment_grading::AssignmentGradingService::update_job_status(
        &pool,
        &input.job_id,
        "running",
    )
    .await?;

    let task = AsyncTaskService::create(
        &pool,
        crate::models::async_task::CreateAsyncTaskInput {
            task_type: String::from("grading"),
            target_id: Some(input.job_id.clone()),
            context_data: Some(serde_json::json!({ "total_assets": assets.len() }).to_string()),
        },
    )
    .await?;
    let pool_clone = pool.inner().clone();
    let workspace_path = get_workspace_path(&app_handle)?;
    let job_id = input.job_id.clone();
    let task_id = task.id.clone();
    let grading_mode = job.grading_mode.clone();
    let answer_key = job.answer_key_json.clone();
    let scoring_rules = job.scoring_rules_json.clone();

    tokio::spawn(async move {
        let run_result = async {
            let mut processed = 0i32;
            let mut failed = 0i32;
            let conflicts = 0i32;

            for asset in assets {
                match services::ocr::OcrService::run_ocr_pipeline(
                    &pool_clone,
                    &asset.id,
                    &job_id,
                    &workspace_path,
                )
                .await
                {
                    Ok(_) => {
                        processed += 1;

                        if grading_mode == "enhanced" {
                            if let Err(error) = services::multimodal_grading::MultimodalGradingService::run_enhanced_grading(
                                &pool_clone,
                                &job_id,
                                &asset.id,
                                answer_key.as_deref(),
                                scoring_rules.as_deref(),
                            )
                            .await
                            {
                                eprintln!("增强批改失败，asset_id={}，error={}", asset.id, error);
                            }
                        }
                    }
                    Err(_) => {
                        failed += 1;
                    }
                }

                let _ = services::assignment_grading::AssignmentGradingService::update_job_progress(
                    &pool_clone,
                    &job_id,
                    processed,
                    failed,
                    conflicts,
                )
                .await;

                let _ = AsyncTaskService::update_progress(&pool_clone, &task_id, &serde_json::json!({ "processed": processed, "failed": failed }).to_string()).await;
            }

            services::assignment_grading::AssignmentGradingService::update_job_status(
                &pool_clone,
                &job_id,
                "completed",
            )
            .await?;
            AsyncTaskService::complete(&pool_clone, &task_id, None).await?;

            let _ = conflicts;

            Ok::<(), AppError>(())
        }
        .await;

        if let Err(error) = run_result {
            let _ = AsyncTaskService::fail(
                &pool_clone,
                &task_id,
                "batch_grading_failed",
                &error.to_string(),
            )
            .await;
        }
    });

    Ok(task)
}

/// 列出批改任务的 OCR 结果。
#[tauri::command]
#[specta::specta]
pub async fn list_job_ocr_results(
    pool: State<'_, SqlitePool>,
    input: ListJobOcrResultsInput,
) -> Result<Vec<AssignmentOcrResult>, AppError> {
    services::assignment_grading::AssignmentGradingService::list_job_ocr_results(
        &pool,
        &input.job_id,
    )
    .await
}

/// 审核单条 OCR 结果。
#[tauri::command]
#[specta::specta]
pub async fn review_ocr_result(
    pool: State<'_, SqlitePool>,
    input: ReviewOcrResultInput,
) -> Result<AssignmentOcrResult, AppError> {
    services::assignment_grading::AssignmentGradingService::review_ocr_result(&pool, input).await
}

/// 批量审核 OCR 结果。
#[tauri::command]
#[specta::specta]
pub async fn batch_review_ocr_results(
    pool: State<'_, SqlitePool>,
    input: BatchReviewOcrResultsInput,
) -> Result<Vec<AssignmentOcrResult>, AppError> {
    services::assignment_grading::AssignmentGradingService::batch_review_ocr_results(&pool, input)
        .await
}

/// 列出冲突 OCR 结果。
#[tauri::command]
#[specta::specta]
pub async fn list_conflict_results(
    pool: State<'_, SqlitePool>,
    input: ListConflictResultsInput,
) -> Result<Vec<AssignmentOcrResult>, AppError> {
    services::multimodal_grading::MultimodalGradingService::detect_conflicts(&pool, &input.job_id)
        .await
}

/// 列出学生错题记录。
#[tauri::command]
#[specta::specta]
pub async fn list_wrong_answers(
    pool: State<'_, SqlitePool>,
    input: ListWrongAnswersInput,
) -> Result<Vec<WrongAnswerRecord>, AppError> {
    services::practice_sheet::PracticeSheetService::list_student_wrong_answers(&pool, input).await
}

/// 解决一条错题记录。
#[tauri::command]
#[specta::specta]
pub async fn resolve_wrong_answer(
    pool: State<'_, SqlitePool>,
    input: ResolveWrongAnswerCommandInput,
) -> Result<WrongAnswerRecord, AppError> {
    services::assignment_grading::AssignmentGradingService::resolve_wrong_answer(&pool, &input.id)
        .await
}

/// 获取练习卷详情。
#[tauri::command]
#[specta::specta]
pub async fn get_practice_sheet(
    pool: State<'_, SqlitePool>,
    input: GetPracticeSheetInput,
) -> Result<PracticeSheet, AppError> {
    services::practice_sheet::PracticeSheetService::get_practice_sheet(&pool, &input.id).await
}

/// 列出学生练习卷。
#[tauri::command]
#[specta::specta]
pub async fn list_student_practice_sheets(
    pool: State<'_, SqlitePool>,
    input: ListStudentPracticeSheetsInput,
) -> Result<Vec<PracticeSheet>, AppError> {
    services::practice_sheet::PracticeSheetService::list_student_practice_sheets(
        &pool,
        &input.student_id,
    )
    .await
}

/// 生成练习卷。
#[tauri::command]
#[specta::specta]
pub async fn generate_practice_sheet(
    app_handle: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: GeneratePracticeSheetInput,
) -> Result<PracticeSheet, AppError> {
    let workspace_path = get_workspace_path(&app_handle)?;
    services::practice_sheet::PracticeSheetService::generate_practice_sheet(
        &pool,
        input,
        &workspace_path,
    )
    .await
}

/// 删除练习卷。
#[tauri::command]
#[specta::specta]
pub async fn delete_practice_sheet(
    pool: State<'_, SqlitePool>,
    input: DeletePracticeSheetInput,
) -> Result<DeleteResponse, AppError> {
    services::practice_sheet::PracticeSheetService::delete_practice_sheet(&pool, &input.id).await?;
    Ok(DeleteResponse { success: true })
}

/// 导出批改结果。
#[tauri::command]
#[specta::specta]
pub async fn export_grading_results(
    pool: State<'_, SqlitePool>,
    input: ExportGradingResultsInput,
) -> Result<ExportGradingResultsResponse, AppError> {
    services::assignment_grading::AssignmentGradingService::export_grading_results(&pool, input)
        .await
}

/// 列出题库条目。
#[tauri::command]
#[specta::specta]
pub async fn list_question_bank(
    pool: State<'_, SqlitePool>,
    input: ListQuestionBankInput,
) -> Result<Vec<QuestionBankItem>, AppError> {
    services::assignment_grading::AssignmentGradingService::list_question_bank(&pool, input).await
}

/// 创建题库条目。
#[tauri::command]
#[specta::specta]
pub async fn create_question_bank_item(
    pool: State<'_, SqlitePool>,
    input: CreateQuestionBankInput,
) -> Result<QuestionBankItem, AppError> {
    services::assignment_grading::AssignmentGradingService::create_question_bank_item(&pool, input)
        .await
}

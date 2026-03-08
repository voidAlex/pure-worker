//! AI 生成编排服务模块
//!
//! 提供家长沟通文案、学期评语、活动公告的 AI 生成能力，
//! 包括单次生成、重新生成、批量生成与进度管理。

use std::collections::HashMap;
use std::path::Path;

use rig::completion::Prompt;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::activity_announcement::{ActivityAnnouncement, CreateActivityAnnouncementInput};
use crate::models::async_task::{AsyncTask, BatchProgress, CreateAsyncTaskInput};
use crate::models::memory_search::MemorySearchInput;
use crate::models::parent_communication::{CreateParentCommunicationInput, ParentCommunication};
use crate::models::semester_comment::{CreateSemesterCommentInput, SemesterComment};
use crate::services::activity_announcement::ActivityAnnouncementService;
use crate::services::async_task::AsyncTaskService;
use crate::services::audit::AuditService;
use crate::services::llm_provider::LlmProviderService;
use crate::services::memory_search::MemorySearchService;
use crate::services::parent_communication::ParentCommunicationService;
use crate::services::prompt_template::PromptTemplateService;
use crate::services::semester_comment::SemesterCommentService;

/// 家长沟通文案 AI 生成结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ParentCommunicationDraft {
    /// 肯定学生表现的段落。
    pub affirmation: String,
    /// 委婉指出的问题。
    pub issue: String,
    /// 给家长的可执行建议。
    pub suggestion: String,
}

/// 学期评语 AI 生成结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SemesterCommentDraft {
    /// 完整评语文本。
    pub comment: String,
    /// 亮点列表。
    pub highlights: Vec<String>,
    /// 待改进方向。
    pub improvements: Vec<String>,
}

/// 活动公告 AI 生成结果。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ActivityAnnouncementDraft {
    /// 完整公告文本。
    pub announcement: String,
    /// 关键要点。
    pub key_points: Vec<String>,
}

/// 生成家长沟通文案输入。
#[derive(Debug, Deserialize, Type)]
pub struct GenerateParentCommInput {
    /// 学生 ID。
    pub student_id: String,
    /// 搜索关键词（可选）。
    pub keyword: Option<String>,
    /// 语气要求（可选，默认"温和正式"）。
    pub tone: Option<String>,
}

/// 重新生成家长沟通文案输入。
#[derive(Debug, Deserialize, Type)]
pub struct RegenerateParentCommInput {
    /// 原记录 ID。
    pub id: String,
    /// 新的语气要求（可选）。
    pub tone: Option<String>,
}

/// 生成单个学期评语输入。
#[derive(Debug, Deserialize, Type)]
pub struct GenerateSemesterCommentInput {
    /// 学生 ID。
    pub student_id: String,
    /// 学期标识（如"2024-2025-1"）。
    pub term: String,
    /// 关联任务 ID（可选）。
    pub task_id: Option<String>,
    /// 已有评语摘要（用于语义去重提示）。
    pub existing_comments_summary: Option<String>,
}

/// 批量生成学期评语输入。
#[derive(Debug, Clone, Deserialize, Serialize, Type)]
pub struct GenerateBatchCommentsInput {
    /// 班级 ID。
    pub class_id: String,
    /// 学期标识。
    pub term: String,
}

/// 生成活动公告输入。
#[derive(Debug, Deserialize, Type)]
pub struct GenerateActivityAnnouncementInput {
    /// 班级 ID。
    pub class_id: String,
    /// 活动标题。
    pub title: String,
    /// 活动主题（可选）。
    pub topic: Option<String>,
    /// 目标受众（parent/student/internal）。
    pub audience: String,
}

/// AI 生成编排服务。
pub struct AiGenerationService;

impl AiGenerationService {
    /// 生成家长沟通文案并保存为草稿记录。
    pub async fn generate_parent_communication(
        pool: &SqlitePool,
        workspace_path: &Path,
        templates_dir: &Path,
        input: GenerateParentCommInput,
    ) -> Result<ParentCommunication, AppError> {
        let student_name = get_student_name(pool, &input.student_id).await?;

        let evidence_result = MemorySearchService::search_evidence(
            pool,
            workspace_path,
            MemorySearchInput {
                keyword: input.keyword.clone(),
                student_id: Some(input.student_id.clone()),
                class_id: None,
                from_date: None,
                to_date: None,
                subject: None,
                source_table: None,
                top_k: Some(10),
                workspace_path: None,
            },
        )
        .await?;
        let evidence_text = format_evidence_text(&evidence_result.items);

        let template = PromptTemplateService::load_template(templates_dir, "parent_communication")?;

        let mut variables = HashMap::new();
        variables.insert(String::from("student_name"), student_name);
        variables.insert(String::from("evidence_text"), evidence_text);
        variables.insert(
            String::from("tone"),
            input
                .tone
                .clone()
                .unwrap_or_else(|| String::from("温和正式")),
        );

        let historical_tone = sqlx::query_scalar::<_, Option<String>>(
            "SELECT tone_preset FROM teacher_profile WHERE is_deleted = 0 ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?
        .flatten();

        if let Some(tone) = historical_tone.filter(|value| !value.trim().is_empty()) {
            variables.insert(String::from("historical_tone"), tone);
        }

        let rendered = PromptTemplateService::render(&template, &variables)?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        let agent =
            LlmProviderService::create_agent(&client, &config.default_model, &rendered.system, 0.7);

        let response: String = agent
            .prompt(&rendered.user)
            .await
            .map_err(|error| AppError::ExternalService(format!("LLM 调用失败：{error}")))?;

        let draft = serde_json::from_str::<ParentCommunicationDraft>(&response)
            .map_err(|error| AppError::ExternalService(format!("LLM 返回格式解析失败：{error}")))?;

        let draft_text = format!(
            "【肯定】{}\n\n【问题】{}\n\n【建议】{}",
            draft.affirmation, draft.issue, draft.suggestion
        );
        let evidence_json = serde_json::to_string(&evidence_result.items).unwrap_or_default();

        let result = ParentCommunicationService::create(
            pool,
            CreateParentCommunicationInput {
                student_id: input.student_id,
                draft: Some(draft_text),
                adopted_text: None,
                status: Some(String::from("draft")),
                evidence_json: Some(evidence_json),
            },
        )
        .await?;

        AuditService::log(
            pool,
            "ai",
            "generate_parent_communication",
            "parent_communication",
            Some(&result.id),
            "medium",
            false,
        )
        .await?;

        Ok(result)
    }

    /// 基于历史记录重新生成家长沟通文案（新建记录，不覆盖旧记录）。
    pub async fn regenerate_parent_communication(
        pool: &SqlitePool,
        workspace_path: &Path,
        templates_dir: &Path,
        input: RegenerateParentCommInput,
    ) -> Result<ParentCommunication, AppError> {
        let existing = ParentCommunicationService::get_by_id(pool, &input.id).await?;
        Self::generate_parent_communication(
            pool,
            workspace_path,
            templates_dir,
            GenerateParentCommInput {
                student_id: existing.student_id,
                keyword: None,
                tone: input.tone,
            },
        )
        .await
    }

    /// 生成单个学期评语并保存为草稿记录。
    pub async fn generate_semester_comment(
        pool: &SqlitePool,
        workspace_path: &Path,
        templates_dir: &Path,
        input: GenerateSemesterCommentInput,
    ) -> Result<SemesterComment, AppError> {
        let student_name = get_student_name(pool, &input.student_id).await?;

        let evidence_result = MemorySearchService::search_evidence(
            pool,
            workspace_path,
            MemorySearchInput {
                keyword: None,
                student_id: Some(input.student_id.clone()),
                class_id: None,
                from_date: None,
                to_date: None,
                subject: None,
                source_table: None,
                top_k: Some(10),
                workspace_path: None,
            },
        )
        .await?;
        let evidence_text = format_evidence_text(&evidence_result.items);
        let evidence_count = evidence_result.returned_count;

        let template = PromptTemplateService::load_template(templates_dir, "semester_comment")?;

        let mut variables = HashMap::new();
        variables.insert(String::from("student_name"), student_name);
        variables.insert(String::from("evidence_text"), evidence_text);
        variables.insert(String::from("term"), input.term.clone());
        variables.insert(String::from("tone"), String::from("客观专业"));

        if let Some(summary) = input
            .existing_comments_summary
            .clone()
            .filter(|value| !value.trim().is_empty())
        {
            variables.insert(String::from("existing_comments_summary"), summary);
        }

        let rendered = PromptTemplateService::render(&template, &variables)?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        let agent =
            LlmProviderService::create_agent(&client, &config.default_model, &rendered.system, 0.7);

        let response: String = agent
            .prompt(&rendered.user)
            .await
            .map_err(|error| AppError::ExternalService(format!("LLM 调用失败：{error}")))?;

        let draft = serde_json::from_str::<SemesterCommentDraft>(&response)
            .map_err(|error| AppError::ExternalService(format!("LLM 返回格式解析失败：{error}")))?;

        let evidence_json = serde_json::to_string(&evidence_result.items).unwrap_or_default();

        let result = SemesterCommentService::create(
            pool,
            CreateSemesterCommentInput {
                student_id: input.student_id,
                task_id: input.task_id,
                term: input.term,
                draft: Some(draft.comment),
                adopted_text: None,
                status: Some(String::from("draft")),
                evidence_json: Some(evidence_json),
                evidence_count: Some(evidence_count as i32),
            },
        )
        .await?;

        AuditService::log(
            pool,
            "ai",
            "generate_semester_comment",
            "semester_comment",
            Some(&result.id),
            "medium",
            false,
        )
        .await?;

        Ok(result)
    }

    /// 启动批量学期评语生成任务。
    pub async fn start_batch_semester_comments(
        pool: &SqlitePool,
        input: GenerateBatchCommentsInput,
    ) -> Result<AsyncTask, AppError> {
        let class_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM classroom WHERE id = ? AND is_deleted = 0",
        )
        .bind(&input.class_id)
        .fetch_one(pool)
        .await?;

        if class_exists == 0 {
            return Err(AppError::InvalidInput(format!(
                "班级不存在或已删除：{}",
                input.class_id
            )));
        }

        let context_data = serde_json::to_string(&input).unwrap_or_default();

        let task = AsyncTaskService::create(
            pool,
            CreateAsyncTaskInput {
                task_type: String::from("batch_semester_comments"),
                target_id: Some(input.class_id.clone()),
                context_data: Some(context_data),
            },
        )
        .await?;

        AuditService::log(
            pool,
            "ai",
            "start_batch_semester_comments",
            "async_task",
            Some(&task.id),
            "medium",
            false,
        )
        .await?;

        Ok(task)
    }

    /// 执行批量学期评语生成任务并持续更新进度。
    pub async fn run_batch_semester_comments(
        pool: &SqlitePool,
        workspace_path: &Path,
        templates_dir: &Path,
        task_id: &str,
        input: GenerateBatchCommentsInput,
    ) -> Result<(), AppError> {
        AsyncTaskService::start(pool, task_id).await?;

        let students = sqlx::query_as::<_, (String, String)>(
            "SELECT id, name FROM student WHERE class_id = ? AND is_deleted = 0",
        )
        .bind(&input.class_id)
        .fetch_all(pool)
        .await?;

        let total = students.len() as i32;
        let mut completed = 0_i32;
        let mut failed = 0_i32;

        for (student_id, student_name) in students {
            let progress = BatchProgress {
                total,
                completed,
                failed,
                current_student_name: Some(student_name.clone()),
            };
            let progress_json = serde_json::to_string(&progress).unwrap_or_default();
            AsyncTaskService::update_progress(pool, task_id, &progress_json).await?;

            let result = Self::generate_semester_comment(
                pool,
                workspace_path,
                templates_dir,
                GenerateSemesterCommentInput {
                    student_id,
                    term: input.term.clone(),
                    task_id: Some(task_id.to_string()),
                    existing_comments_summary: None,
                },
            )
            .await;

            if let Err(error) = result {
                failed += 1;
                let detail_json = serde_json::json!({
                    "task_id": task_id,
                    "class_id": input.class_id,
                    "student_name": student_name,
                    "error": error.to_string(),
                })
                .to_string();

                let _ = AuditService::log_with_detail(
                    pool,
                    "ai",
                    "generate_semester_comment_failed",
                    "semester_comment",
                    None,
                    "medium",
                    false,
                    Some(&detail_json),
                )
                .await;
            } else {
                completed += 1;
            }
        }

        let final_progress = BatchProgress {
            total,
            completed,
            failed,
            current_student_name: None,
        };
        let final_progress_json = serde_json::to_string(&final_progress).unwrap_or_default();
        AsyncTaskService::update_progress(pool, task_id, &final_progress_json).await?;

        if failed == 0 {
            AsyncTaskService::complete(pool, task_id, None).await?;
        } else {
            let summary = format!("完成 {completed}/{total}，失败 {failed}");
            AsyncTaskService::complete(pool, task_id, Some(&summary)).await?;
        }

        Ok(())
    }

    /// 生成活动公告并保存为草稿记录。
    pub async fn generate_activity_announcement(
        pool: &SqlitePool,
        templates_dir: &Path,
        input: GenerateActivityAnnouncementInput,
    ) -> Result<ActivityAnnouncement, AppError> {
        let template =
            PromptTemplateService::load_template(templates_dir, "activity_announcement")?;

        let mut variables = HashMap::new();
        variables.insert(String::from("title"), input.title.clone());
        variables.insert(
            String::from("topic"),
            input.topic.clone().unwrap_or_else(|| input.title.clone()),
        );
        variables.insert(String::from("audience"), input.audience.clone());
        variables.insert(String::from("tone"), String::from("正式清晰"));

        let rendered = PromptTemplateService::render(&template, &variables)?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        let agent =
            LlmProviderService::create_agent(&client, &config.default_model, &rendered.system, 0.7);

        let response: String = agent
            .prompt(&rendered.user)
            .await
            .map_err(|error| AppError::ExternalService(format!("LLM 调用失败：{error}")))?;

        let draft = serde_json::from_str::<ActivityAnnouncementDraft>(&response)
            .map_err(|error| AppError::ExternalService(format!("LLM 返回格式解析失败：{error}")))?;

        let result = ActivityAnnouncementService::create(
            pool,
            CreateActivityAnnouncementInput {
                class_id: input.class_id,
                title: input.title,
                topic: input.topic,
                audience: Some(input.audience),
                draft: Some(draft.announcement),
                adopted_text: None,
                template_id: None,
                status: Some(String::from("draft")),
            },
        )
        .await?;

        AuditService::log(
            pool,
            "ai",
            "generate_activity_announcement",
            "activity_announcement",
            Some(&result.id),
            "medium",
            false,
        )
        .await?;

        Ok(result)
    }
}

/// 获取学生姓名。
async fn get_student_name(pool: &SqlitePool, student_id: &str) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>("SELECT name FROM student WHERE id = ? AND is_deleted = 0")
        .bind(student_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("学生不存在或已删除：{student_id}")))
}

/// 将证据列表格式化为文本。
fn format_evidence_text(items: &[crate::models::memory_search::EvidenceItem]) -> String {
    if items.is_empty() {
        return String::from("暂无相关记录");
    }

    items
        .iter()
        .map(|item| item.content.as_str())
        .collect::<Vec<_>>()
        .join("\n---\n")
}

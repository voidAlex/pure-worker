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
use crate::models::ai_param_preset::AiParamPreset;
use crate::models::async_task::{AsyncTask, BatchProgress, CreateAsyncTaskInput};
use crate::models::memory_search::MemorySearchInput;
use crate::models::parent_communication::{CreateParentCommunicationInput, ParentCommunication};
use crate::models::semester_comment::{CreateSemesterCommentInput, SemesterComment};
use crate::services::activity_announcement::ActivityAnnouncementService;
use crate::services::ai_param_preset::AiParamPresetService;
use crate::services::async_task::AsyncTaskService;
use crate::services::audit::AuditService;
use crate::services::desensitize::DesensitizeService;
use crate::services::llm_provider::LlmProviderService;
use crate::services::memory_search::MemorySearchService;
use crate::services::parent_communication::ParentCommunicationService;
use crate::services::prompt_template::PromptTemplateService;
use crate::services::semester_comment::SemesterCommentService;
use crate::services::template_file::TemplateFileService;

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
    /// 校本模板 ID（可选）。
    pub template_id: Option<String>,
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

        // M3-020: 注入成绩趋势和标签
        let score_trend_text = get_score_trend_text(pool, &input.student_id).await?;
        variables.insert(String::from("score_trend"), score_trend_text);

        let tags_text = get_student_tags_text(pool, &input.student_id).await?;
        variables.insert(String::from("student_tags"), tags_text);

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
        let safe_user_prompt =
            DesensitizeService::desensitize_if_enabled(pool, &rendered.user).await?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        // 获取当前激活的参数预设
        let preset = AiParamPresetService::get_active_preset(pool)
            .await
            .unwrap_or_else(|_| AiParamPreset::default_balanced());
        let temperature = preset.temperature;
        let agent = LlmProviderService::create_agent(
            &client,
            &config.default_model,
            &rendered.system,
            temperature,
        );

        let response: String = agent
            .prompt(&safe_user_prompt)
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
        let safe_user_prompt =
            DesensitizeService::desensitize_if_enabled(pool, &rendered.user).await?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        // 获取当前激活的参数预设
        let preset = AiParamPresetService::get_active_preset(pool)
            .await
            .unwrap_or_else(|_| AiParamPreset::default_balanced());
        let temperature = preset.temperature;
        let agent = LlmProviderService::create_agent(
            &client,
            &config.default_model,
            &rendered.system,
            temperature,
        );

        let response: String = agent
            .prompt(&safe_user_prompt)
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

            let student_id_clone = student_id.clone();
            let result = Self::generate_semester_comment(
                pool,
                workspace_path,
                templates_dir,
                GenerateSemesterCommentInput {
                    student_id: student_id_clone,
                    term: input.term.clone(),
                    task_id: Some(task_id.to_string()),
                    existing_comments_summary: None,
                },
            )
            .await;
            // M3-031: 语义去重检查
            if let Ok(comment) = &result {
                let draft_text = comment.draft.as_deref().unwrap_or("");
                if !draft_text.is_empty() {
                    match check_comment_duplicate(pool, &student_id, draft_text, 0.75).await {
                        Ok((is_duplicate, similarity)) => {
                            if is_duplicate {
                                // 标记为跳过（去重）
                                failed += 1;
                                let detail_json = serde_json::json!({
                                    "task_id": task_id,
                                    "class_id": input.class_id,
                                    "student_name": student_name,
                                    "reason": "semantic_duplicate",
                                    "similarity": similarity,
                                })
                                .to_string();

                                if let Err(e) = AuditService::log_with_detail(
                                    pool,
                                    "ai",
                                    "semester_comment_skipped_duplicate",
                                    "semester_comment",
                                    None,
                                    "low",
                                    false,
                                    Some(&detail_json),
                                )
                                .await
                                {
                                    eprintln!("[审计日志] 记录评语去重跳过审计失败：{e}");
                                }
                                continue;
                            }
                        }
                        Err(e) => {
                            // 去重检查失败，记录但不影响主流程
                            let err_msg = format!("duplicate_check_failed: {}", e);
                            if let Err(audit_err) = AuditService::log(
                                pool,
                                "ai",
                                &err_msg,
                                "semester_comment",
                                None,
                                "low",
                                false,
                            )
                            .await
                            {
                                eprintln!("[审计日志] 记录去重检查失败审计失败：{audit_err}");
                            }
                        }
                    }
                }
            }

            if let Err(error) = &result {
                failed += 1;
                let detail_json = serde_json::json!({
                    "task_id": task_id,
                    "class_id": input.class_id,
                    "student_name": student_name,
                    "error": error.to_string(),
                })
                .to_string();

                if let Err(e) = AuditService::log_with_detail(
                    pool,
                    "ai",
                    "generate_semester_comment_failed",
                    "semester_comment",
                    None,
                    "medium",
                    false,
                    Some(&detail_json),
                )
                .await
                {
                    eprintln!("[审计日志] 记录评语生成失败审计失败：{e}");
                }
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

        // 如果指定了校本模板 ID，加载模板文件路径并读取内容注入到提示词变量中
        if let Some(template_id) = input.template_id.as_deref() {
            let template_file = TemplateFileService::get_by_id(pool, template_id).await?;
            if template_file.enabled == 1 {
                match std::fs::read_to_string(&template_file.file_path) {
                    Ok(content) => {
                        variables.insert(String::from("school_template"), content);
                    }
                    Err(error) => {
                        // 模板文件读取失败不阻断生成，仅跳过
                        eprintln!("[AiGeneration] 校本模板文件读取失败：{error}");
                    }
                }
            }
        }

        let rendered = PromptTemplateService::render(&template, &variables)?;
        let safe_user_prompt =
            DesensitizeService::desensitize_if_enabled(pool, &rendered.user).await?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        // 获取当前激活的参数预设
        let preset = AiParamPresetService::get_active_preset(pool)
            .await
            .unwrap_or_else(|_| AiParamPreset::default_balanced());
        let temperature = preset.temperature;
        let agent = LlmProviderService::create_agent(
            &client,
            &config.default_model,
            &rendered.system,
            temperature,
        );

        let response: String = agent
            .prompt(&safe_user_prompt)
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
                template_id: input.template_id,
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

/// 获取学生标签列表文本。
async fn get_student_tags_text(pool: &SqlitePool, student_id: &str) -> Result<String, AppError> {
    let tags: Vec<String> = sqlx::query_scalar(
        "SELECT tag_name FROM student_tag WHERE student_id = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT 10",
    )
    .bind(student_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .filter(|tag: &String| !tag.trim().is_empty())
    .collect();
    if tags.is_empty() {
        Ok(String::from("暂无标签"))
    } else {
        Ok(tags.join("、"))
    }
}

/// 获取学生成绩趋势文本。
async fn get_score_trend_text(pool: &SqlitePool, student_id: &str) -> Result<String, AppError> {
    #[derive(sqlx::FromRow)]
    struct ScoreRow {
        exam_name: String,
        subject: String,
        score: f64,
        full_score: f64,
        exam_date: String,
    }

    let records = sqlx::query_as::<_, ScoreRow>(
        "SELECT exam_name, subject, score, full_score, exam_date FROM score_record WHERE student_id = ? AND is_deleted = 0 ORDER BY exam_date DESC LIMIT 10",
    )
    .bind(student_id)
    .fetch_all(pool)
    .await?;

    if records.is_empty() {
        return Ok(String::from("暂无成绩记录"));
    }

    // 按学科分组计算趋势
    let mut subject_map: std::collections::HashMap<String, Vec<(String, f64)>> =
        std::collections::HashMap::new();
    for record in records {
        let percentage = if record.full_score > 0.0 {
            (record.score / record.full_score * 100.0).round() as i32
        } else {
            0
        };
        subject_map
            .entry(record.subject.clone())
            .or_default()
            .push((
                format!(
                    "{}({}%) [{}]",
                    record.exam_name,
                    percentage,
                    record.exam_date.split('T').next().unwrap_or("未知日期")
                ),
                record.score,
            ));
    }

    // 生成趋势文本
    let mut lines = Vec::new();
    for (subject, exams) in subject_map {
        if exams.len() >= 2 {
            let recent = &exams[0];
            let previous = &exams[exams.len() - 1];
            let trend = if recent.1 > previous.1 {
                "↑上升"
            } else if recent.1 < previous.1 {
                "↓下降"
            } else {
                "→稳定"
            };
            lines.push(format!("【{}】{}，整体趋势{}", subject, recent.0, trend));
        } else if !exams.is_empty() {
            lines.push(format!("【{}】{}", subject, exams[0].0));
        }
    }

    if lines.is_empty() {
        Ok(String::from("暂无成绩记录"))
    } else {
        Ok(lines.join("\n"))
    }
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

/// 计算两个文本的字符 3-gram Jaccard 相似度。
/// 返回值范围 [0.0, 1.0]，值越大表示越相似。
fn calculate_text_similarity(text1: &str, text2: &str) -> f64 {
    if text1.is_empty() || text2.is_empty() {
        return 0.0;
    }

    // 生成字符 3-gram 集合
    fn get_ngrams(text: &str, n: usize) -> std::collections::HashSet<String> {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() < n {
            return std::collections::HashSet::from([text.to_string()]);
        }
        chars
            .windows(n)
            .map(|window| window.iter().collect::<String>())
            .collect()
    }

    let ngrams1 = get_ngrams(text1, 3);
    let ngrams2 = get_ngrams(text2, 3);

    if ngrams1.is_empty() || ngrams2.is_empty() {
        return 0.0;
    }

    // 计算 Jaccard 相似度
    let intersection = ngrams1.intersection(&ngrams2).count();
    let union = ngrams1.union(&ngrams2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// 检查新评语是否与已有评语高度相似。
/// 返回 (is_duplicate, max_similarity)
async fn check_comment_duplicate(
    pool: &SqlitePool,
    student_id: &str,
    new_comment: &str,
    threshold: f64,
) -> Result<(bool, f64), AppError> {
    // 查询该学生本学期已有的评语
    let existing_comments: Vec<String> = sqlx::query_scalar(
        "SELECT COALESCE(adopted_text, draft) FROM semester_comment WHERE student_id = ? AND status IN ('draft', 'adopted') AND is_deleted = 0 ORDER BY created_at DESC LIMIT 5",
    )
    .bind(student_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .filter(|c: &String| !c.trim().is_empty())
    .collect();

    if existing_comments.is_empty() {
        return Ok((false, 0.0));
    }

    // 计算与每个已有评语的相似度，取最大值
    let mut max_similarity = 0.0;
    for existing in existing_comments {
        let similarity = calculate_text_similarity(new_comment, existing.as_str());
        if similarity > max_similarity {
            max_similarity = similarity;
        }
    }

    Ok((max_similarity >= threshold, max_similarity))
}

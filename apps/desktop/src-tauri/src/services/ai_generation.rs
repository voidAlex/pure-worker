//! AI 生成编排服务模块
//!
//! 提供家长沟通文案、学期评语、活动公告的 AI 生成能力，
//! 包括单次生成、重新生成、批量生成与进度管理。
//!
//! 采用 Prompt-based Skills 架构（Claude 官方模式）：
//! - Skills 通过 body_content 注入 LLM 上下文
//! - LLM 根据 description 自然语言匹配触发
//! - LLM 自主决策执行步骤

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
use crate::services::skill::SkillService;

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

        // Prompt-based Skills: 注入完整 skill body_content
        let skills_context = format_enabled_skills_context(pool).await?;
        variables.insert(String::from("skills_context"), skills_context);

        let rendered = PromptTemplateService::render(&template, &variables)?;
        let safe_user_prompt =
            DesensitizeService::desensitize_if_enabled(pool, &rendered.user).await?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        let preset = AiParamPresetService::get_active_preset(pool)
            .await
            .unwrap_or_else(|_| AiParamPreset::default_balanced());
        let temperature = preset.temperature;

        // Prompt-based 模式：不使用 tools，完全依赖 LLM 自主决策
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

        let template = PromptTemplateService::load_template(templates_dir, "semester_comment")?;

        let mut variables = HashMap::new();
        variables.insert(String::from("student_name"), student_name);
        variables.insert(String::from("evidence_text"), evidence_text);
        variables.insert(String::from("term"), input.term.clone());

        if let Some(ref summary) = input.existing_comments_summary {
            variables.insert(
                String::from("existing_comments_summary"),
                summary.to_string(),
            );
        }

        // Prompt-based Skills: 注入完整 skill body_content
        let skills_context = format_enabled_skills_context(pool).await?;
        variables.insert(String::from("skills_context"), skills_context);

        let rendered = PromptTemplateService::render(&template, &variables)?;
        let safe_user_prompt =
            DesensitizeService::desensitize_if_enabled(pool, &rendered.user).await?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        let preset = AiParamPresetService::get_active_preset(pool)
            .await
            .unwrap_or_else(|_| AiParamPreset::default_balanced());
        let temperature = preset.temperature;

        // Prompt-based 模式
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

        // 语义去重检查
        let _is_duplicate = if let Some(summary) = &input.existing_comments_summary {
            check_semantic_duplicate(&response, summary).await?
        } else {
            (false, 0.0)
        };

        let draft: SemesterCommentDraft = serde_json::from_str(&response)
            .map_err(|error| AppError::ExternalService(format!("LLM 返回格式解析失败：{error}")))?;

        let draft_text = draft.comment;
        let evidence_json = serde_json::to_string(&evidence_result.items).unwrap_or_default();

        let result = SemesterCommentService::create(
            pool,
            CreateSemesterCommentInput {
                student_id: input.student_id,
                term: input.term,
                draft: Some(draft_text),
                adopted_text: None,
                status: Some(String::from("draft")),
                evidence_json: Some(evidence_json),
                evidence_count: Some(evidence_result.items.len() as i32),
                task_id: input.task_id,
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

    /// 启动批量学期评语任务。
    pub async fn start_batch_semester_comments(
        pool: &SqlitePool,
        input: GenerateBatchCommentsInput,
    ) -> Result<AsyncTask, AppError> {
        let class_students =
            crate::services::student::StudentService::list(pool, Some(&input.class_id)).await?;

        let total = class_students.len();
        if total == 0 {
            return Err(AppError::InvalidInput(String::from("该班级没有学生")));
        }

        let task = AsyncTaskService::create(
            pool,
            CreateAsyncTaskInput {
                task_type: String::from("generate_batch_semester_comments"),
                target_id: None,
                context_data: None,
            },
        )
        .await?;

        Ok(task)
    }

    /// 执行批量学期评语生成。
    pub async fn run_batch_semester_comments(
        pool: &SqlitePool,
        workspace_path: &Path,
        templates_dir: &Path,
        task_id: &str,
        input: GenerateBatchCommentsInput,
    ) -> Result<(), AppError> {
        let class_students =
            crate::services::student::StudentService::list(pool, Some(&input.class_id)).await?;

        let total = class_students.len();
        let mut completed = 0;
        let mut failed = 0;

        // 收集已生成的评语用于去重
        let mut existing_comments: Vec<String> = Vec::new();

        for student in class_students {
            let existing_summary = if existing_comments.is_empty() {
                None
            } else {
                Some(existing_comments.join("\n"))
            };

            let result = Self::generate_semester_comment(
                pool,
                workspace_path,
                templates_dir,
                GenerateSemesterCommentInput {
                    student_id: student.id,
                    term: input.term.clone(),
                    task_id: Some(task_id.to_string()),
                    existing_comments_summary: existing_summary,
                },
            )
            .await;

            match result {
                Ok(comment) => {
                    completed += 1;
                    if let Some(draft) = comment.draft {
                        existing_comments.push(draft);
                    }
                }
                Err(_) => {
                    failed += 1;
                }
            }

            let progress = BatchProgress {
                completed,
                failed,
                total: total as i32,
                current_student_name: Some(student.name.clone()),
            };
            let progress_json = serde_json::to_string(&progress).unwrap_or_default();
            let _ = AsyncTaskService::update_progress(pool, task_id, &progress_json).await;
        }

        let _status = if failed == 0 {
            String::from("completed")
        } else if completed == 0 {
            String::from("failed")
        } else {
            String::from("partial")
        };
        let _ = AsyncTaskService::complete(pool, task_id, None).await;

        Ok(())
    }

    /// 生成活动公告并保存为草稿记录。
    pub async fn generate_activity_announcement(
        pool: &SqlitePool,
        _workspace_path: &Path,
        templates_dir: &Path,
        _template_file_dir: &Path,
        input: GenerateActivityAnnouncementInput,
    ) -> Result<ActivityAnnouncement, AppError> {
        let class_name = get_class_name(pool, &input.class_id).await?;

        // 加载校本模板（如有）
        let template_content = if let Some(template_id) = &input.template_id {
            crate::services::template_file::TemplateFileService::get_by_id(pool, template_id)
                .await
                .ok()
                .map(|t| t.file_path)
        } else {
            None
        };

        let template =
            PromptTemplateService::load_template(templates_dir, "activity_announcement")?;

        let mut variables = HashMap::new();
        variables.insert(String::from("class_name"), class_name);
        variables.insert(String::from("title"), input.title.clone());
        variables.insert(
            String::from("topic"),
            input.topic.clone().unwrap_or_else(|| input.title.clone()),
        );
        variables.insert(String::from("audience"), input.audience.clone());

        if let Some(content) = template_content {
            variables.insert(String::from("template_content"), content);
        }

        // Prompt-based Skills: 注入完整 skill body_content
        let skills_context = format_enabled_skills_context(pool).await?;
        variables.insert(String::from("skills_context"), skills_context);

        let rendered = PromptTemplateService::render(&template, &variables)?;
        let safe_user_prompt =
            DesensitizeService::desensitize_if_enabled(pool, &rendered.user).await?;

        let config = LlmProviderService::get_active_config(pool).await?;
        let client = LlmProviderService::create_client(&config)?;
        let preset = AiParamPresetService::get_active_preset(pool)
            .await
            .unwrap_or_else(|_| AiParamPreset::default_balanced());
        let temperature = preset.temperature;

        // Prompt-based 模式
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

        let draft: ActivityAnnouncementDraft = serde_json::from_str(&response)
            .map_err(|error| AppError::ExternalService(format!("LLM 返回格式解析失败：{error}")))?;

        let draft_text = draft.announcement;

        let result = ActivityAnnouncementService::create(
            pool,
            CreateActivityAnnouncementInput {
                class_id: input.class_id,
                title: input.title,
                topic: input.topic,
                audience: Some(input.audience),
                draft: Some(draft_text),
                adopted_text: None,
                status: Some(String::from("draft")),
                template_id: input.template_id,
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
    let record = sqlx::query_as::<_, crate::models::student::Student>(
        "SELECT id, name, class_id, gender, enrollment_date, created_at, updated_at, is_deleted FROM students WHERE id = ? AND is_deleted = 0",
    )
    .bind(student_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("学生不存在：{student_id}")))?;
    Ok(record.name)
}

/// 获取班级名称。
async fn get_class_name(pool: &SqlitePool, class_id: &str) -> Result<String, AppError> {
    let record = sqlx::query_as::<_, crate::models::classroom::Classroom>(
        "SELECT id, grade, class_name, subject, teacher_id, is_deleted, created_at, updated_at FROM classrooms WHERE id = ? AND is_deleted = 0",
    )
    .bind(class_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("班级不存在：{class_id}")))?;
    Ok(record.class_name)
}

/// 格式化证据文本。
fn format_evidence_text(items: &[crate::models::memory_search::EvidenceItem]) -> String {
    if items.is_empty() {
        return String::from("暂无相关观察记录");
    }
    items
        .iter()
        .map(|item| {
            format!(
                "- [{}] {}: {}",
                item.created_at,
                item.subject.as_deref().unwrap_or("其他"),
                item.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 获取成绩趋势文本。
async fn get_score_trend_text(pool: &SqlitePool, student_id: &str) -> Result<String, AppError> {
    let scores = crate::services::score_record::ScoreRecordService::list_student_scores(
        pool, student_id, None, None, None,
    )
    .await?;

    if scores.is_empty() {
        return Ok(String::from("暂无成绩记录"));
    }

    let trend_text = scores
        .iter()
        .map(|s| format!("- {} {}: {}分", s.exam_date, s.subject, s.score))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(trend_text)
}

/// 获取学生标签文本。
async fn get_student_tags_text(pool: &SqlitePool, student_id: &str) -> Result<String, AppError> {
    let tags =
        crate::services::student_tag::StudentTagService::list_by_student(pool, student_id).await?;

    if tags.is_empty() {
        return Ok(String::from("暂无标签"));
    }

    let tags_text = tags
        .iter()
        .map(|t| format!("- {}", t.tag_name))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(tags_text)
}

/// 格式化启用的 Skills 为 Prompt 上下文（Prompt-based 模式）。
///
/// 将 Skill 的完整 body_content 注入，让 LLM 根据 description 自主匹配触发。
async fn format_enabled_skills_context(pool: &SqlitePool) -> Result<String, AppError> {
    let skills = SkillService::list_skills(pool).await?;
    let enabled_skills: Vec<_> = skills
        .into_iter()
        .filter(|s| s.status.as_deref() == Some("enabled"))
        .filter(|s| s.body_content.is_some()) // 只包含已加载 body_content 的技能
        .collect();

    if enabled_skills.is_empty() {
        return Ok(String::from("无可用技能"));
    }

    let mut context = String::from("## 可用技能\n\n");
    context.push_str(
        "当你需要执行以下任务时，请根据【触发条件】匹配用户需求，并按照【执行指令】自主完成。\n\n",
    );
    context.push_str("---\n\n");

    for skill in enabled_skills {
        let name = &skill.name;
        let description = skill.description.as_deref().unwrap_or("无描述");
        let body = skill.body_content.as_deref().unwrap_or("无执行指令");
        let allowed_tools = skill.allowed_tools.as_deref().unwrap_or("无限制");

        context.push_str(&format!("### Skill: {}\n\n", name));
        context.push_str(&format!("**触发条件**: {}\n\n", description));

        if allowed_tools != "无限制" {
            context.push_str(&format!("**允许使用的工具**: {}\n\n", allowed_tools));
        }

        context.push_str("**执行指令**:\n");
        context.push_str(body);
        context.push_str("\n\n---\n\n");
    }

    Ok(context)
}

/// 语义去重检查。
async fn check_semantic_duplicate(
    new_comment: &str,
    existing_summary: &str,
) -> Result<(bool, f64), AppError> {
    // 简化实现：使用文本相似度
    let existing_comments: Vec<String> = existing_summary
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect();

    if existing_comments.is_empty() {
        return Ok((false, 0.0));
    }

    let threshold = 0.85;
    let mut max_similarity = 0.0;

    for existing in existing_comments {
        let similarity = calculate_text_similarity(new_comment, existing.as_str());
        if similarity > max_similarity {
            max_similarity = similarity;
        }
    }

    Ok((max_similarity >= threshold, max_similarity))
}

/// 计算两段文本的相似度（简化版 Jaccard 相似度）。
fn calculate_text_similarity(a: &str, b: &str) -> f64 {
    let a_words: std::collections::HashSet<String> =
        a.split_whitespace().map(|w| w.to_lowercase()).collect();
    let b_words: std::collections::HashSet<String> =
        b.split_whitespace().map(|w| w.to_lowercase()).collect();

    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

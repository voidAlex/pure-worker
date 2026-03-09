//! 练习卷生成服务模块
//!
//! 提供错题检索、题目模板化、参数扰动生成变体题和 Word 练习卷导出能力。

use std::path::Path;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use chrono::Utc;
use docx_rs::{Docx, Paragraph, Run};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::assignment_grading::{
    GeneratePracticeSheetInput, ListQuestionBankInput, PracticeSheet, QuestionBankItem,
    WrongAnswerRecord,
};
use crate::services::audit::AuditService;

/// 练习卷题目条目（用于 Word 生成）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QuestionItem {
    number: i32,
    stem: String,
    knowledge_point: String,
    difficulty: String,
}

/// 练习卷答案条目（用于 Word 生成）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AnswerItem {
    number: i32,
    answer: String,
    explanation: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TemplateParamsConfig {
    params: Vec<TemplateParam>,
    formula: Option<String>,
    stem_template: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TemplateParam {
    name: String,
    #[serde(rename = "type")]
    param_type: String,
    value: serde_json::Value,
    min: Option<f64>,
    max: Option<f64>,
}

/// 练习卷生成服务，负责错题重组和专属练习卷的生成与导出。
pub struct PracticeSheetService;

impl PracticeSheetService {
    /// 生成练习卷：入库生成中状态、检索题目、导出 Word、回写结果并审计。
    pub async fn generate_practice_sheet(
        pool: &SqlitePool,
        input: GeneratePracticeSheetInput,
        workspace_path: &Path,
    ) -> Result<PracticeSheet, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let question_count = input.question_count.unwrap_or(10);
        if question_count <= 0 {
            return Err(AppError::InvalidInput(String::from("题目数量必须大于 0")));
        }

        let knowledge_points_json = input
            .knowledge_points
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| AppError::InvalidInput(format!("知识点参数序列化失败：{error}")))?;

        sqlx::query(
            "INSERT INTO practice_sheet (id, student_id, title, knowledge_points_json, difficulty, question_count, questions_json, answers_json, file_path, answer_file_path, status, task_id, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, 'generating', NULL, 0, ?, ?)",
        )
        .bind(&id)
        .bind(&input.student_id)
        .bind(&input.title)
        .bind(&knowledge_points_json)
        .bind(&input.difficulty)
        .bind(question_count)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let build_result = async {
            let wrong_answers = sqlx::query_as::<_, WrongAnswerRecord>(
                "SELECT id, student_id, job_id, ocr_result_id, question_no, knowledge_point, difficulty, student_answer, correct_answer, score, full_score, error_type, is_resolved, is_deleted, created_at, updated_at FROM wrong_answer_record WHERE student_id = ? AND is_deleted = 0 AND created_at >= datetime('now', '-30 days') ORDER BY created_at DESC LIMIT ?",
            )
            .bind(&input.student_id)
            .bind(question_count)
            .fetch_all(pool)
            .await?;

            let mut points = input.knowledge_points.clone().unwrap_or_default();
            if points.is_empty() {
                points = wrong_answers
                    .iter()
                    .filter_map(|x| x.knowledge_point.clone())
                    .collect();
                points.sort();
                points.dedup();
            }

            let mut selected = Self::fetch_relevant_questions(
                pool,
                &points,
                input.difficulty.as_deref(),
                question_count,
            )
            .await?;

            if selected.len() < question_count as usize {
                let lacking = question_count as usize - selected.len();
                let candidate_ids: Vec<String> =
                    wrong_answers.iter().map(|x| x.question_no.clone()).collect();
                if !candidate_ids.is_empty() {
                    let mut builder = sqlx::QueryBuilder::new(
                        "SELECT id, source, knowledge_point, difficulty, stem, answer, explanation, is_deleted, created_at, updated_at, question_type, subject, grade, tags_json, template_params_json, parent_id FROM question_bank WHERE is_deleted = 0 AND id IN (",
                    );
                    {
                        let mut separated = builder.separated(", ");
                        for item in &candidate_ids {
                            separated.push_bind(item);
                        }
                    }
                    builder.push(") ORDER BY RANDOM() LIMIT ").push_bind(lacking as i32);
                    let supplements = builder
                        .build_query_as::<QuestionBankItem>()
                        .fetch_all(pool)
                        .await?;
                    for item in supplements {
                        if !selected.iter().any(|q| q.id == item.id) {
                            selected.push(item);
                        }
                    }
                }
            }

            let selected: Vec<QuestionBankItem> =
                selected.into_iter().take(question_count as usize).collect();
            if selected.is_empty() {
                return Err(AppError::NotFound(String::from("未找到可用于生成练习卷的题目")));
            }

            let variants: Vec<QuestionBankItem> = selected.iter().map(Self::perturb_question).collect();
            let questions: Vec<QuestionItem> = variants
                .iter()
                .enumerate()
                .map(|(idx, q)| QuestionItem {
                    number: idx as i32 + 1,
                    stem: q.stem.clone(),
                    knowledge_point: q
                        .knowledge_point
                        .clone()
                        .unwrap_or_else(|| String::from("未标注")),
                    difficulty: q
                        .difficulty
                        .clone()
                        .unwrap_or_else(|| String::from("未标注")),
                })
                .collect();
            let answers: Vec<AnswerItem> = variants
                .iter()
                .enumerate()
                .map(|(idx, q)| AnswerItem {
                    number: idx as i32 + 1,
                    answer: q.answer.clone().unwrap_or_else(|| String::from("暂无标准答案")),
                    explanation: q.explanation.clone().unwrap_or_else(|| String::from("暂无解析")),
                })
                .collect();

            let questions_json = serde_json::to_string(&questions)
                .map_err(|error| AppError::Internal(format!("题目序列化失败：{error}")))?;
            let answers_json = serde_json::to_string(&answers)
                .map_err(|error| AppError::Internal(format!("答案序列化失败：{error}")))?;

            let output_dir = workspace_path.join("practice_sheets");
            let (file_path, answer_file_path) =
                Self::generate_word_document(&input.title, &questions, &answers, &output_dir).await?;

            sqlx::query(
                "UPDATE practice_sheet SET questions_json = ?, answers_json = ?, file_path = ?, answer_file_path = ?, status = 'completed', updated_at = ? WHERE id = ? AND is_deleted = 0",
            )
            .bind(&questions_json)
            .bind(&answers_json)
            .bind(&file_path)
            .bind(&answer_file_path)
            .bind(Utc::now().to_rfc3339())
            .bind(&id)
            .execute(pool)
            .await?;

            AuditService::log(
                pool,
                "system",
                "generate_practice_sheet",
                "practice_sheet",
                Some(&id),
                "medium",
                false,
            )
            .await?;

            Ok(())
        }
        .await;

        if let Err(error) = build_result {
            let _ = sqlx::query(
                "UPDATE practice_sheet SET status = 'failed', updated_at = ? WHERE id = ? AND is_deleted = 0",
            )
            .bind(Utc::now().to_rfc3339())
            .bind(&id)
            .execute(pool)
            .await;
            return Err(error);
        }

        Self::get_practice_sheet(pool, &id).await
    }

    /// 根据 ID 查询练习卷详情。
    pub async fn get_practice_sheet(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<PracticeSheet, AppError> {
        let sheet = sqlx::query_as::<_, PracticeSheet>(
            "SELECT id, student_id, title, knowledge_points_json, difficulty, question_count, questions_json, answers_json, file_path, answer_file_path, status, task_id, is_deleted, created_at, updated_at FROM practice_sheet WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("练习卷不存在：{id}")))?;
        Ok(sheet)
    }

    /// 查询学生练习卷列表，按创建时间倒序。
    pub async fn list_student_practice_sheets(
        pool: &SqlitePool,
        student_id: &str,
    ) -> Result<Vec<PracticeSheet>, AppError> {
        let sheets = sqlx::query_as::<_, PracticeSheet>(
            "SELECT id, student_id, title, knowledge_points_json, difficulty, question_count, questions_json, answers_json, file_path, answer_file_path, status, task_id, is_deleted, created_at, updated_at FROM practice_sheet WHERE student_id = ? AND is_deleted = 0 ORDER BY created_at DESC",
        )
        .bind(student_id)
        .fetch_all(pool)
        .await?;
        Ok(sheets)
    }

    /// 软删除练习卷。
    pub async fn delete_practice_sheet(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE practice_sheet SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("练习卷不存在：{id}")));
        }

        AuditService::log(
            pool,
            "system",
            "delete_practice_sheet",
            "practice_sheet",
            Some(id),
            "medium",
            false,
        )
        .await?;
        Ok(())
    }

    /// 生成题目文档和答案文档并返回路径。
    async fn generate_word_document(
        title: &str,
        questions: &[QuestionItem],
        answers: &[AnswerItem],
        output_dir: &Path,
    ) -> Result<(String, String), AppError> {
        let title = title.to_string();
        let questions = questions.to_vec();
        let answers = answers.to_vec();
        let output_dir = output_dir.to_path_buf();

        tokio::task::spawn_blocking(move || -> Result<(String, String), AppError> {
            std::fs::create_dir_all(&output_dir)
                .map_err(|error| AppError::FileOperation(format!("创建输出目录失败：{error}")))?;

            let uuid = Uuid::new_v4().to_string();
            let q_path = output_dir.join(format!("practice_{uuid}.docx"));
            let a_path = output_dir.join(format!("practice_{uuid}_answers.docx"));

            let mut q_docx = Docx::new().add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(&title).bold().size(36)),
            );
            for item in &questions {
                let line = format!(
                    "第{}题（知识点：{}，难度：{}）\n{}",
                    item.number, item.knowledge_point, item.difficulty, item.stem
                );
                q_docx = q_docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
                q_docx = q_docx.add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text("--------------------")),
                );
            }

            let q_file = std::fs::File::create(&q_path)
                .map_err(|error| AppError::FileOperation(format!("创建题目文档失败：{error}")))?;
            q_docx
                .build()
                .pack(q_file)
                .map_err(|error| AppError::FileOperation(format!("写入题目文档失败：{error}")))?;

            let mut a_docx = Docx::new().add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(format!("{}（答案）", title))
                        .bold()
                        .size(36),
                ),
            );
            for item in &answers {
                let line = format!(
                    "第{}题\n答案：{}\n解析：{}",
                    item.number, item.answer, item.explanation
                );
                a_docx = a_docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
                a_docx = a_docx.add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text("--------------------")),
                );
            }

            let a_file = std::fs::File::create(&a_path)
                .map_err(|error| AppError::FileOperation(format!("创建答案文档失败：{error}")))?;
            a_docx
                .build()
                .pack(a_file)
                .map_err(|error| AppError::FileOperation(format!("写入答案文档失败：{error}")))?;

            Ok((
                q_path.to_string_lossy().to_string(),
                a_path.to_string_lossy().to_string(),
            ))
        })
        .await
        .map_err(|error| AppError::Internal(format!("Word 文档生成任务失败：{error}")))?
    }

    /// 按知识点、难度随机检索题库。
    async fn fetch_relevant_questions(
        pool: &SqlitePool,
        knowledge_points: &[String],
        difficulty: Option<&str>,
        limit: i32,
    ) -> Result<Vec<QuestionBankItem>, AppError> {
        let query_input = ListQuestionBankInput {
            source: None,
            knowledge_point: knowledge_points.first().cloned(),
            question_type: None,
            subject: None,
            difficulty: difficulty.map(str::to_string),
            grade: None,
            limit: Some(limit),
        };

        let mut builder = sqlx::QueryBuilder::new(
            "SELECT id, source, knowledge_point, difficulty, stem, answer, explanation, is_deleted, created_at, updated_at, question_type, subject, grade, tags_json, template_params_json, parent_id FROM question_bank WHERE is_deleted = 0",
        );
        if !knowledge_points.is_empty() {
            builder.push(" AND knowledge_point IN (");
            {
                let mut separated = builder.separated(", ");
                for point in knowledge_points {
                    separated.push_bind(point);
                }
            }
            builder.push(")");
        }
        if let Some(level) = query_input.difficulty.as_deref() {
            builder.push(" AND difficulty = ").push_bind(level);
        }
        builder
            .push(" ORDER BY RANDOM() LIMIT ")
            .push_bind(query_input.limit.unwrap_or(limit));

        let rows = builder
            .build_query_as::<QuestionBankItem>()
            .fetch_all(pool)
            .await?;
        Ok(rows)
    }

    /// 基于模板参数生成题目变体。
    ///
    /// 逻辑说明：
    /// 1. 优先解析 `template_params_json`，若缺失或解析失败则退化为克隆题目；
    /// 2. 对 int/float 数值参数执行 ±10%~30% 的扰动，并强制约束在 [min, max]；
    /// 3. 用新参数替换 `stem_template` 中 `{param}` 占位符生成新题干；
    /// 4. 若存在 `formula`，按“从左到右”的基础四则运算重新计算答案；
    /// 5. 无论是否成功扰动，都保持 `parent_id=原题ID` 且重新生成变体题 ID。
    fn perturb_question(question: &QuestionBankItem) -> QuestionBankItem {
        let mut cloned = question.clone();
        cloned.parent_id = Some(question.id.clone());
        cloned.id = Uuid::new_v4().to_string();

        let Some(raw_template) = question.template_params_json.as_deref() else {
            return cloned;
        };

        let Ok(config) = serde_json::from_str::<TemplateParamsConfig>(raw_template) else {
            return cloned;
        };

        let now_seed = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let mut numeric_values: HashMap<String, f64> = HashMap::new();
        let mut display_values: HashMap<String, String> = HashMap::new();

        for param in &config.params {
            let Some(original_value) = Self::json_to_f64(&param.value) else {
                continue;
            };

            let min = param.min.unwrap_or(original_value);
            let max = param.max.unwrap_or(original_value);
            if min > max {
                continue;
            }

            let mut hasher = DefaultHasher::new();
            question.id.hash(&mut hasher);
            param.name.hash(&mut hasher);
            now_seed.hash(&mut hasher);
            let hashed = hasher.finish();

            let pct_bucket = (hashed % 21) as f64;
            let ratio = 0.10 + pct_bucket / 100.0;
            let direction = if ((hashed >> 8) & 1) == 0 { 1.0 } else { -1.0 };
            let perturbed = (original_value * (1.0 + direction * ratio)).clamp(min, max);

            match param.param_type.as_str() {
                "int" => {
                    let rounded = perturbed.round();
                    let int_value = if rounded.is_finite() {
                        if rounded > i64::MAX as f64 {
                            i64::MAX
                        } else if rounded < i64::MIN as f64 {
                            i64::MIN
                        } else {
                            rounded as i64
                        }
                    } else {
                        continue;
                    };
                    numeric_values.insert(param.name.clone(), int_value as f64);
                    display_values.insert(param.name.clone(), int_value.to_string());
                }
                "float" => {
                    let float_value = (perturbed * 100.0).round() / 100.0;
                    numeric_values.insert(param.name.clone(), float_value);
                    display_values.insert(param.name.clone(), format!("{float_value:.2}"));
                }
                _ => {
                    continue;
                }
            }
        }

        if let Some(stem_template) = config.stem_template.as_deref() {
            let mut new_stem = stem_template.to_string();
            for (name, value) in &display_values {
                let placeholder = format!("{{{name}}}");
                new_stem = new_stem.replace(&placeholder, value);
            }
            cloned.stem = new_stem;
        }

        if let Some(formula) = config.formula.as_deref() {
            if let Some(result) = Self::evaluate_formula(formula, &numeric_values) {
                let answer = if (result.fract()).abs() < f64::EPSILON {
                    (result as i64).to_string()
                } else {
                    let rounded = (result * 100.0).round() / 100.0;
                    format!("{rounded:.2}")
                };
                cloned.answer = Some(answer);
            }
        }

        cloned
    }

    fn json_to_f64(value: &serde_json::Value) -> Option<f64> {
        if let Some(v) = value.as_f64() {
            return Some(v);
        }
        if let Some(text) = value.as_str() {
            return text.parse::<f64>().ok();
        }
        None
    }

    fn evaluate_formula(formula: &str, values: &HashMap<String, f64>) -> Option<f64> {
        #[derive(Debug)]
        enum Token {
            Number(f64),
            Op(char),
        }

        let mut tokens: Vec<Token> = Vec::new();
        let chars: Vec<char> = formula.chars().collect();
        let mut idx = 0usize;

        while idx < chars.len() {
            let ch = chars[idx];
            if ch.is_whitespace() {
                idx += 1;
                continue;
            }

            if matches!(ch, '+' | '-' | '*' | '/') {
                tokens.push(Token::Op(ch));
                idx += 1;
                continue;
            }

            if ch.is_ascii_digit() || ch == '.' {
                let start = idx;
                idx += 1;
                while idx < chars.len() && (chars[idx].is_ascii_digit() || chars[idx] == '.') {
                    idx += 1;
                }
                let text: String = chars[start..idx].iter().collect();
                let number = text.parse::<f64>().ok()?;
                tokens.push(Token::Number(number));
                continue;
            }

            if ch.is_alphabetic() || ch == '_' {
                let start = idx;
                idx += 1;
                while idx < chars.len() && (chars[idx].is_alphanumeric() || chars[idx] == '_') {
                    idx += 1;
                }
                let name: String = chars[start..idx].iter().collect();
                let number = values.get(&name).copied()?;
                tokens.push(Token::Number(number));
                continue;
            }

            return None;
        }

        let mut iter = tokens.into_iter();
        let mut acc = match iter.next() {
            Some(Token::Number(v)) => v,
            _ => return None,
        };

        while let Some(token) = iter.next() {
            let op = match token {
                Token::Op(c) => c,
                Token::Number(_) => return None,
            };
            let rhs = match iter.next() {
                Some(Token::Number(v)) => v,
                _ => return None,
            };

            acc = match op {
                '+' => acc + rhs,
                '-' => acc - rhs,
                '*' => acc * rhs,
                '/' => {
                    if rhs.abs() < f64::EPSILON {
                        return None;
                    }
                    acc / rhs
                }
                _ => return None,
            };
        }

        Some(acc)
    }

    /// 查询学生错题记录，支持知识点/任务 ID/是否解决等筛选条件。
    pub async fn list_student_wrong_answers(
        pool: &SqlitePool,
        input: crate::models::assignment_grading::ListWrongAnswersInput,
    ) -> Result<Vec<WrongAnswerRecord>, AppError> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT id, student_id, job_id, ocr_result_id, question_no, knowledge_point, difficulty, student_answer, correct_answer, score, full_score, error_type, is_resolved, is_deleted, created_at, updated_at FROM wrong_answer_record WHERE is_deleted = 0 AND created_at >= datetime('now', '-30 days')",
        );

        if let Some(student_id) = &input.student_id {
            qb.push(" AND student_id = ").push_bind(student_id);
        }
        if let Some(job_id) = &input.job_id {
            qb.push(" AND job_id = ").push_bind(job_id);
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
}

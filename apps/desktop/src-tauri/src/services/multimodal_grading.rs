//! 多模态批改服务模块
//!
//! 该模块负责作业批改中的多模态增强流程：
//! 1) 发起基于图像与答案要点的 LLM 批改（M4 Phase 1 为占位实现）；
//! 2) 融合 OCR 分数与 LLM 分数，输出可供教师确认的建议分；
//! 3) 识别 OCR 与 LLM 评分冲突并打标，进入人工复核；
//! 4) 当多模态能力不可用时自动降级到 OCR + 规则判分路径。

use chrono::Utc;
use rig::completion::Prompt;
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::assignment_grading::AssignmentOcrResult;
use crate::services::audit::AuditService;
use crate::services::llm_provider::LlmProviderService;

pub struct MultimodalGradingService;

impl MultimodalGradingService {
    /// 使用多模态 LLM 执行批改。
    ///
    /// 说明：
    /// 1) 读取当前激活的 AI 配置并创建 rig-core 客户端；
    /// 2) 生成标准化批改提示词，请求 LLM 返回 JSON 数组；
    /// 3) 将题目级分数与反馈回写到 `assignment_ocr_result`。
    pub async fn grade_with_llm(
        pool: &SqlitePool,
        asset_id: &str,
        job_id: &str,
        answer_key_json: Option<&str>,
        scoring_rules_json: Option<&str>,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        if asset_id.trim().is_empty() || job_id.trim().is_empty() {
            return Err(AppError::InvalidInput(
                "参数无效：asset_id 和 job_id 不能为空".into(),
            ));
        }

        let detail_json = json!({
            "trace_id": Uuid::new_v4().to_string(),
            "asset_id": asset_id,
            "job_id": job_id,
            "stage": "llm_grading_attempt",
            "has_answer_key": answer_key_json.is_some(),
            "has_scoring_rules": scoring_rules_json.is_some(),
        });

        if let Err(e) = AuditService::log_with_detail(
            pool,
            "system",
            "multimodal_llm_grading_attempt",
            "assignment_asset",
            Some(asset_id),
            "medium",
            false,
            Some(&detail_json.to_string()),
        )
        .await
        {
            eprintln!("[审计日志] 记录多模态批改尝试审计失败：{e}");
        }

        let config = match LlmProviderService::get_active_config(pool).await {
            Ok(config) => config,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::ExternalService(
                    "多模态 LLM 尚未配置，请在系统设置中配置 AI 模型".into(),
                ));
            }
            Err(error) => return Err(error),
        };
        let client = LlmProviderService::create_client(&config)?;

        let system_prompt = String::from(
            "你是一个专业的作业批改助手。请根据提供的标准答案和评分规则，对学生的作答进行评分。\
             你必须只返回 JSON 数组，不要输出任何额外说明或 Markdown 代码块。\
             返回格式固定为：\
             [{\"question_no\":\"1\",\"score\":8.0,\"feedback\":\"...\"}]。\
             score 必须是数字，feedback 必须是中文自然语言。",
        );

        let mut user_prompt_sections = vec![
            format!("asset_id: {asset_id}"),
            format!("job_id: {job_id}"),
            String::from("请基于以下信息输出逐题评分 JSON："),
        ];
        if let Some(answer_key) = answer_key_json {
            user_prompt_sections.push(format!("answer_key_json: {answer_key}"));
        }
        if let Some(scoring_rules) = scoring_rules_json {
            user_prompt_sections.push(format!("scoring_rules_json: {scoring_rules}"));
        }
        user_prompt_sections.push(String::from(
            "请确保 question_no 与作答题号对应；若无法判断题号，请使用原始题号文本。",
        ));
        let user_prompt = user_prompt_sections.join("\n");

        let agent =
            LlmProviderService::create_agent(&client, &config.default_model, &system_prompt, 0.3);
        let response: String = agent
            .prompt(&user_prompt)
            .await
            .map_err(|e| AppError::ExternalService(format!("LLM 批改调用失败：{e}")))?;

        let grading_items: Vec<serde_json::Value> = serde_json::from_str(&response)
            .map_err(|e| AppError::ExternalService(format!("LLM 批改结果解析失败：{e}")))?;

        let fallback_student_id = sqlx::query_scalar::<_, Option<String>>(
            "SELECT student_id FROM assignment_ocr_result WHERE asset_id = ? AND job_id = ? AND is_deleted = 0 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(asset_id)
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询回写学生信息失败：{e}")))?
        .flatten();

        for item in &grading_items {
            let Some(question_no) = item.get("question_no").and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let score = item.get("score").and_then(serde_json::Value::as_f64);
            let feedback = item
                .get("feedback")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned);
            let feedback_ref = feedback.as_deref();

            if score.is_none() && feedback.is_none() {
                continue;
            }

            let updated_at = Utc::now().to_rfc3339();

            let update_result = sqlx::query(
                "UPDATE assignment_ocr_result
                 SET multimodal_score = ?, multimodal_feedback = ?, updated_at = ?
                 WHERE asset_id = ? AND job_id = ? AND question_no = ? AND is_deleted = 0",
            )
            .bind(score)
            .bind(feedback_ref)
            .bind(&updated_at)
            .bind(asset_id)
            .bind(job_id)
            .bind(question_no)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("回写 LLM 批改结果失败：{e}")))?;

            if update_result.rows_affected() == 0 {
                let Some(student_id) = fallback_student_id.as_deref() else {
                    continue;
                };
                let now = Utc::now().to_rfc3339();
                sqlx::query(
                    "INSERT INTO assignment_ocr_result
                     (id, asset_id, job_id, student_id, question_no, answer_text, confidence, score, created_at, multimodal_score, multimodal_feedback, conflict_flag, review_status, is_deleted, updated_at)
                     VALUES (?, ?, ?, ?, ?, NULL, NULL, NULL, ?, ?, ?, 0, 'pending', 0, ?)",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(asset_id)
                .bind(job_id)
                .bind(student_id)
                .bind(question_no)
                .bind(&now)
                .bind(score)
                .bind(feedback_ref)
                .bind(&now)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(format!("插入 LLM 批改结果失败：{e}")))?;
            }
        }

        let updated_rows = sqlx::query_as::<_, AssignmentOcrResult>(
            "SELECT * FROM assignment_ocr_result WHERE asset_id = ? AND job_id = ? AND is_deleted = 0",
        )
        .bind(asset_id)
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询 LLM 批改结果失败：{e}")))?;

        Ok(updated_rows)
    }

    /// 融合 OCR 与 LLM 判分结果，并对冲突或低置信度样本打标。
    ///
    /// 说明：
    /// 1) 先进行 OCR/LLM 分数融合并写入冲突标记；
    /// 2) 再根据 OCR 置信度阈值（< 0.85）统一标记为 `needs_review`。
    pub async fn fuse_results(
        pool: &SqlitePool,
        asset_id: &str,
        job_id: &str,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        if asset_id.trim().is_empty() || job_id.trim().is_empty() {
            return Err(AppError::InvalidInput(
                "参数无效：asset_id 和 job_id 不能为空".into(),
            ));
        }

        let rows = sqlx::query_as::<_, AssignmentOcrResult>(
            "SELECT * FROM assignment_ocr_result WHERE asset_id = ? AND job_id = ? AND is_deleted = 0",
        )
        .bind(asset_id)
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询 OCR 结果失败：{e}")))?;

        if rows.is_empty() {
            return Err(AppError::NotFound("未找到可融合的 OCR 批改结果".into()));
        }

        for row in &rows {
            let ocr_score = row.score;
            let llm_score = row.multimodal_score;

            let (merged_score, conflict_flag, fusion_mode) = match (ocr_score, llm_score) {
                (Some(ocr), Some(llm)) => {
                    let full_score = row.final_score.unwrap_or(ocr.max(llm)).max(1.0);
                    let diff = (ocr - llm).abs();
                    let conflict = if diff > full_score * 0.2 { 1 } else { 0 };
                    let merged = if conflict == 1 {
                        Some(ocr)
                    } else {
                        Some((ocr + llm) / 2.0)
                    };
                    let mode = if conflict == 1 {
                        "ocr_llm_conflict"
                    } else {
                        "ocr_llm_agree"
                    };
                    (merged, conflict, mode)
                }
                (Some(ocr), None) => (Some(ocr), 0, "ocr_only_degraded"),
                (None, Some(llm)) => (Some(llm), 0, "llm_only_fill"),
                (None, None) => (None, 0, "no_score"),
            };

            let updated_at = Utc::now().to_rfc3339();

            sqlx::query(
                "UPDATE assignment_ocr_result
                 SET score = ?, conflict_flag = ?, updated_at = ?
                 WHERE id = ? AND is_deleted = 0",
            )
            .bind(merged_score)
            .bind(conflict_flag)
            .bind(&updated_at)
            .bind(&row.id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("更新融合结果失败：{e}")))?;

            if row.confidence.is_some_and(|c| c < 0.85) {
                sqlx::query(
                    "UPDATE assignment_ocr_result
                     SET review_status = 'needs_review', updated_at = ?
                     WHERE id = ? AND is_deleted = 0",
                )
                .bind(&updated_at)
                .bind(&row.id)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(format!("更新低置信度复核标记失败：{e}")))?;
            }

            let detail_json = json!({
                "trace_id": Uuid::new_v4().to_string(),
                "asset_id": asset_id,
                "job_id": job_id,
                "record_id": row.id,
                "fusion_mode": fusion_mode,
                "conflict_flag": conflict_flag,
                "ocr_score": ocr_score,
                "llm_score": llm_score,
                "merged_score": merged_score,
            });

            if let Err(e) = AuditService::log_with_detail(
                pool,
                "system",
                "multimodal_fusion_update",
                "assignment_ocr_result",
                Some(&row.id),
                "medium",
                false,
                Some(&detail_json.to_string()),
            )
            .await
            {
                eprintln!("[审计日志] 记录多模态融合更新审计失败：{e}");
            }
        }

        let updated_rows = sqlx::query_as::<_, AssignmentOcrResult>(
            "SELECT * FROM assignment_ocr_result WHERE asset_id = ? AND job_id = ? AND is_deleted = 0",
        )
        .bind(asset_id)
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询融合后结果失败：{e}")))?;

        Ok(updated_rows)
    }

    /// 查询冲突与低置信度样本，供教师进行人工复核。
    pub async fn detect_conflicts(
        pool: &SqlitePool,
        job_id: &str,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        if job_id.trim().is_empty() {
            return Err(AppError::InvalidInput("参数无效：job_id 不能为空".into()));
        }

        let rows = sqlx::query_as::<_, AssignmentOcrResult>(
            "SELECT * FROM assignment_ocr_result
             WHERE job_id = ? AND (conflict_flag = 1 OR review_status = 'needs_review') AND is_deleted = 0
             ORDER BY created_at DESC",
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询冲突结果失败：{e}")))?;

        Ok(rows)
    }

    /// 检查多模态 LLM 是否可用。
    ///
    /// 说明：通过读取激活 AI 配置判断可用性，读取失败则视为不可用。
    pub async fn check_llm_availability(pool: &SqlitePool) -> bool {
        LlmProviderService::get_active_config(pool).await.is_ok()
    }

    /// 执行增强批改主流程：可用时走 OCR+LLM 融合，不可用时自动降级。
    ///
    /// 说明：降级路径会执行规则判分，避免返回空结果导致后续流程缺失数据。
    pub async fn run_enhanced_grading(
        pool: &SqlitePool,
        asset_id: &str,
        job_id: &str,
        answer_key_json: Option<&str>,
        scoring_rules_json: Option<&str>,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        if asset_id.trim().is_empty() || job_id.trim().is_empty() {
            return Err(AppError::InvalidInput(
                "参数无效：asset_id 和 job_id 不能为空".into(),
            ));
        }

        if Self::check_llm_availability(pool).await {
            match Self::grade_with_llm(pool, asset_id, job_id, answer_key_json, scoring_rules_json)
                .await
            {
                Ok(_) => {
                    let detail_json = json!({
                        "trace_id": Uuid::new_v4().to_string(),
                        "asset_id": asset_id,
                        "job_id": job_id,
                        "mode": "enhanced",
                        "strategy": "OCR + 多模态 LLM 融合",
                    });
                    if let Err(e) = AuditService::log_with_detail(
                        pool,
                        "system",
                        "multimodal_enhanced_grading",
                        "assignment_asset",
                        Some(asset_id),
                        "medium",
                        false,
                        Some(&detail_json.to_string()),
                    )
                    .await
                    {
                        eprintln!("[审计日志] 记录增强批改审计失败：{e}");
                    }

                    Self::fuse_results(pool, asset_id, job_id).await
                }
                Err(err) => {
                    let detail_json = json!({
                        "trace_id": Uuid::new_v4().to_string(),
                        "asset_id": asset_id,
                        "job_id": job_id,
                        "mode": "degraded_after_llm_error",
                        "strategy": "当多模态 LLM 未配置、不可用或超时时，自动回落 OCR + 规则判分",
                        "reason": format!("{err:?}"),
                    });
                    if let Err(e) = AuditService::log_with_detail(
                        pool,
                        "system",
                        "multimodal_grading_degraded",
                        "assignment_asset",
                        Some(asset_id),
                        "low",
                        false,
                        Some(&detail_json.to_string()),
                    )
                    .await
                    {
                        eprintln!("[审计日志] 记录批改降级审计失败：{e}");
                    }

                    Self::fuse_results(pool, asset_id, job_id)
                        .await
                        .map_err(|e| AppError::TaskExecution(format!("降级融合失败：{e:?}")))
                }
            }
        } else {
            let detail_json = json!({
                "trace_id": Uuid::new_v4().to_string(),
                "asset_id": asset_id,
                "job_id": job_id,
                "mode": "degraded",
                "strategy": "当多模态 LLM 未配置、不可用或超时时，自动回落 OCR + 规则判分",
            });

            if let Err(e) = AuditService::log_with_detail(
                pool,
                "system",
                "multimodal_grading_degraded",
                "assignment_asset",
                Some(asset_id),
                "low",
                false,
                Some(&detail_json.to_string()),
            )
            .await
            {
                eprintln!("[审计日志] 记录批改降级审计失败：{e}");
            }

            let rows = sqlx::query_as::<_, AssignmentOcrResult>(
                "SELECT * FROM assignment_ocr_result WHERE asset_id = ? AND job_id = ? AND is_deleted = 0",
            )
            .bind(asset_id)
            .bind(job_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("查询降级判分数据失败：{e}")))?;

            let answer_key_value = match answer_key_json {
                Some(raw) => {
                    Some(serde_json::from_str::<serde_json::Value>(raw).map_err(|e| {
                        AppError::InvalidInput(format!("标准答案 JSON 格式无效：{e}"))
                    })?)
                }
                None => None,
            };
            let scoring_rules_value = match scoring_rules_json {
                Some(raw) => {
                    Some(serde_json::from_str::<serde_json::Value>(raw).map_err(|e| {
                        AppError::InvalidInput(format!("评分规则 JSON 格式无效：{e}"))
                    })?)
                }
                None => None,
            };

            for row in &rows {
                let Some(question_no) = row.question_no.as_deref() else {
                    continue;
                };
                let Some(answer_text) = row.answer_text.as_deref() else {
                    continue;
                };
                if answer_text.trim().is_empty() {
                    continue;
                }
                let Some(answer_keys) = answer_key_value.as_ref() else {
                    continue;
                };

                let correct_answer = answer_keys
                    .get(question_no)
                    .and_then(serde_json::Value::as_str);
                let Some(correct_answer_text) = correct_answer else {
                    continue;
                };

                let full_score = scoring_rules_value
                    .as_ref()
                    .and_then(|value| value.get(question_no))
                    .and_then(|value| value.get("full_score"))
                    .and_then(serde_json::Value::as_f64)
                    .or_else(|| {
                        scoring_rules_value
                            .as_ref()
                            .and_then(|value| value.get("full_score"))
                            .and_then(serde_json::Value::as_f64)
                    })
                    .unwrap_or(10.0);

                let score = if answer_text.contains(correct_answer_text) {
                    full_score
                } else {
                    0.0
                };
                let updated_at = Utc::now().to_rfc3339();

                sqlx::query(
                    "UPDATE assignment_ocr_result
                     SET score = ?, confidence = ?, updated_at = ?
                     WHERE id = ? AND is_deleted = 0",
                )
                .bind(score)
                .bind(0.5_f64)
                .bind(&updated_at)
                .bind(&row.id)
                .execute(pool)
                .await
                .map_err(|e| AppError::Database(format!("降级规则判分写回失败：{e}")))?;
            }

            let detail_json = json!({
                "trace_id": Uuid::new_v4().to_string(),
                "asset_id": asset_id,
                "job_id": job_id,
                "mode": "degraded_rule_based",
                "strategy": "使用标准答案进行简单包含匹配，命中判满分，未命中判 0 分",
            });

            if let Err(e) = AuditService::log_with_detail(
                pool,
                "system",
                "multimodal_rule_based_scoring",
                "assignment_asset",
                Some(asset_id),
                "low",
                false,
                Some(&detail_json.to_string()),
            )
            .await
            {
                eprintln!("[审计日志] 记录规则判分审计失败：{e}");
            }

            let updated_rows = sqlx::query_as::<_, AssignmentOcrResult>(
                "SELECT * FROM assignment_ocr_result WHERE asset_id = ? AND job_id = ? AND is_deleted = 0",
            )
            .bind(asset_id)
            .bind(job_id)
            .fetch_all(pool)
            .await
            .map_err(|e| AppError::Database(format!("查询降级判分结果失败：{e}")))?;

            Ok(updated_rows)
        }
    }
}

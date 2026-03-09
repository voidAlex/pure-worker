//! 多模态批改服务模块
//!
//! 该模块负责作业批改中的多模态增强流程：
//! 1) 发起基于图像与答案要点的 LLM 批改（M4 Phase 1 为占位实现）；
//! 2) 融合 OCR 分数与 LLM 分数，输出可供教师确认的建议分；
//! 3) 识别 OCR 与 LLM 评分冲突并打标，进入人工复核；
//! 4) 当多模态能力不可用时自动降级到 OCR + 规则判分路径。

use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::assignment_grading::AssignmentOcrResult;
use crate::services::audit::AuditService;

pub struct MultimodalGradingService;

impl MultimodalGradingService {
    /// 使用多模态 LLM 执行批改（M4 Phase 1 占位实现）。
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

        let _ = AuditService::log_with_detail(
            pool,
            "system",
            "multimodal_llm_grading_attempt",
            "assignment_asset",
            Some(asset_id),
            "medium",
            false,
            Some(&detail_json.to_string()),
        )
        .await;

        // TODO(M4-Phase2): 通过 rig-core 调用多模态模型，发送图像 + 答案要点 + 评分规则，
        // 返回题目级分数与反馈并写回 assignment_ocr_result.multimodal_score / multimodal_feedback。
        Err(AppError::ExternalService(
            "多模态 LLM 尚未配置，请在系统设置中配置 AI 模型".into(),
        ))
    }

    /// 融合 OCR 与 LLM 判分结果，并对冲突样本打标。
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

            let _ = AuditService::log_with_detail(
                pool,
                "system",
                "multimodal_fusion_update",
                "assignment_ocr_result",
                Some(&row.id),
                "medium",
                false,
                Some(&detail_json.to_string()),
            )
            .await;
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

    /// 查询冲突样本，供教师进行人工复核。
    pub async fn detect_conflicts(
        pool: &SqlitePool,
        job_id: &str,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        if job_id.trim().is_empty() {
            return Err(AppError::InvalidInput("参数无效：job_id 不能为空".into()));
        }

        let rows = sqlx::query_as::<_, AssignmentOcrResult>(
            "SELECT * FROM assignment_ocr_result
             WHERE job_id = ? AND conflict_flag = 1 AND is_deleted = 0
             ORDER BY created_at DESC",
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("查询冲突结果失败：{e}")))?;

        Ok(rows)
    }

    /// 检查多模态 LLM 是否可用。
    pub async fn check_llm_availability(_pool: &SqlitePool) -> bool {
        // TODO(M4-Phase2): 检查系统 AI 配置中是否存在可用的多模态模型与有效密钥。
        false
    }

    /// 执行增强批改主流程：可用时走 OCR+LLM 融合，不可用时自动降级。
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
                    let _ = AuditService::log_with_detail(
                        pool,
                        "system",
                        "multimodal_enhanced_grading",
                        "assignment_asset",
                        Some(asset_id),
                        "medium",
                        false,
                        Some(&detail_json.to_string()),
                    )
                    .await;

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
                    let _ = AuditService::log_with_detail(
                        pool,
                        "system",
                        "multimodal_grading_degraded",
                        "assignment_asset",
                        Some(asset_id),
                        "low",
                        false,
                        Some(&detail_json.to_string()),
                    )
                    .await;

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

            let _ = AuditService::log_with_detail(
                pool,
                "system",
                "multimodal_grading_degraded",
                "assignment_asset",
                Some(asset_id),
                "low",
                false,
                Some(&detail_json.to_string()),
            )
            .await;

            Ok(Vec::new())
        }
    }
}

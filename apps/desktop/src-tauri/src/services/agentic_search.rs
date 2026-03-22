//! Agentic Search 编排器
//!
//! 协调多源检索、证据去重排序，为 AI 对话提供上下文增强。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::error::AppError;
use crate::models::agentic_search::{
    AgenticSearchInput, AgenticSearchResult, EvidenceSource, EvidenceSourceType, SearchCacheEntry,
    SearchContext,
};
use crate::models::memory_search::{EvidenceItem, MemorySearchInput};
use crate::services::intent_classifier::{IntentClassification, IntentClassifier};
use crate::services::memory_search::MemorySearchService;
use crate::services::student::StudentService;

/// 搜索任务类型别名
///
/// 用于并行搜索任务的 JoinHandle 类型
pub type SearchTask =
    tokio::task::JoinHandle<Result<Vec<(EvidenceItem, EvidenceSourceType)>, AppError>>;

/// Agentic Search 编排器
pub struct AgenticSearchOrchestrator {
    classifier: IntentClassifier,
    cache: Arc<Mutex<HashMap<String, SearchCacheEntry>>>,
}

/// 运行时检索阶段结果
#[derive(Debug, Clone)]
pub struct SearchStageResult {
    pub evidence: Vec<EvidenceItem>,
    pub search_summary_json: String,
    pub reasoning_summary: String,
}

impl AgenticSearchOrchestrator {
    /// 创建新的编排器
    pub fn new() -> Self {
        Self {
            classifier: IntentClassifier::new(),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 执行 Agentic Search
    ///
    /// 步骤：
    /// 1. 解析意图和实体
    /// 2. 并行搜索多数据源
    /// 3. 证据去重和排序
    /// 4. 生成带引用的结果
    pub async fn search(
        &self,
        pool: &SqlitePool,
        workspace_path: &Path,
        input: AgenticSearchInput,
    ) -> Result<AgenticSearchResult, AppError> {
        // 检查缓存
        if !input.force_refresh.unwrap_or(false) {
            if let Some(cached) = self.get_cached_result(&input).await {
                return Ok(cached);
            }
        }

        // 步骤 1: 意图分类和实体提取
        let classification = self.classifier.classify(&input.query);

        // 如果不需要检索证据，直接返回空结果
        if !classification.needs_evidence {
            return Ok(AgenticSearchResult::empty());
        }

        // 步骤 2: 构建搜索上下文
        let context = self.build_search_context(pool, &classification).await?;

        // 步骤 3: 并行搜索多数据源
        let evidence = self
            .parallel_search(pool, workspace_path, &context, &classification)
            .await?;

        // 步骤 4: 证据去重和排序
        let ranked_evidence = self.deduplicate_and_rank(evidence);

        // 步骤 5: 组装结果
        let result = self.assemble_result(ranked_evidence, &classification);

        // 缓存结果
        self.cache_result(&input, &result).await;

        Ok(result)
    }

    /// 以运行时阶段方式执行检索
    pub async fn search_stage(
        &self,
        pool: &SqlitePool,
        workspace_path: &Path,
        input: AgenticSearchInput,
    ) -> Result<SearchStageResult, AppError> {
        let result = self.search(pool, workspace_path, input).await?;
        let evidence = result
            .evidence_sources
            .iter()
            .map(|source| EvidenceItem {
                content: source.full_content.clone(),
                source_table: source.source_type.description().to_string(),
                source_id: source.source_id.clone(),
                student_id: String::new(),
                class_id: None,
                created_at: source
                    .created_at
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339()),
                score: f64::from(source.relevance_score),
                file_path: None,
                subject: None,
            })
            .collect::<Vec<EvidenceItem>>();

        let search_summary_json = serde_json::json!({
            "conclusion": result.conclusion,
            "confidence_score": result.confidence_score,
            "risk_warnings": result.risk_warnings,
            "source_count": result.evidence_sources.len()
        })
        .to_string();

        let reasoning_summary = if result.evidence_sources.is_empty() {
            String::from("未检索到证据，保持空摘要")
        } else {
            format!(
                "共检索到 {} 条证据，已完成去重排序",
                result.evidence_sources.len()
            )
        };

        Ok(SearchStageResult {
            evidence,
            search_summary_json,
            reasoning_summary,
        })
    }

    /// 从缓存获取结果
    async fn get_cached_result(&self, input: &AgenticSearchInput) -> Option<AgenticSearchResult> {
        let cache_key = self.generate_cache_key(input);
        let cache = self.cache.lock().await;

        if let Some(entry) = cache.get(&cache_key) {
            // 缓存有效期：5分钟
            let max_age = chrono::Duration::minutes(5);
            if Utc::now() - entry.cached_at < max_age {
                return Some(entry.result.clone());
            }
        }

        None
    }

    /// 缓存结果
    async fn cache_result(&self, input: &AgenticSearchInput, result: &AgenticSearchResult) {
        let cache_key = self.generate_cache_key(input);
        let mut cache = self.cache.lock().await;

        cache.insert(
            cache_key,
            SearchCacheEntry {
                query_hash: self.generate_cache_key(input),
                result: result.clone(),
                cached_at: Utc::now(),
            },
        );

        // 清理过期缓存（保持最多100条）
        if cache.len() > 100 {
            let keys_to_remove: Vec<String> = cache
                .iter()
                .filter(|(_, entry)| Utc::now() - entry.cached_at > chrono::Duration::minutes(10))
                .map(|(k, _)| k.clone())
                .collect();

            for key in keys_to_remove {
                cache.remove(&key);
            }
        }
    }

    /// 生成缓存键
    fn generate_cache_key(&self, input: &AgenticSearchInput) -> String {
        let session_prefix = input.session_id.as_deref().unwrap_or("default");
        format!("{}:{}", session_prefix, input.query)
    }

    /// 构建搜索上下文
    async fn build_search_context(
        &self,
        pool: &SqlitePool,
        classification: &IntentClassification,
    ) -> Result<SearchContext, AppError> {
        let mut context = SearchContext {
            student_id: None,
            class_id: None,
            subject: classification.entities.subject.clone(),
            from_date: classification.entities.from_date.clone(),
            to_date: classification.entities.to_date.clone(),
            keywords: classification.entities.keywords.clone(),
        };

        // 解析学生姓名到ID
        if !classification.entities.student_names.is_empty() {
            let student_name = &classification.entities.student_names[0];
            let student_id: Option<String> = sqlx::query_scalar(
                "SELECT id FROM student WHERE name = ? AND is_deleted = 0 LIMIT 1",
            )
            .bind(student_name)
            .fetch_optional(pool)
            .await?;

            if let Some(id) = student_id {
                context.student_id = Some(id.clone());

                // 获取学生班级
                let class_id: Option<String> =
                    sqlx::query_scalar("SELECT class_id FROM student WHERE id = ?")
                        .bind(&id)
                        .fetch_optional(pool)
                        .await?;
                context.class_id = class_id;
            }
        }

        // 解析班级名称到ID
        if context.class_id.is_none() {
            if let Some(class_name) = &classification.entities.class_name {
                let class_id: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM classroom WHERE name = ? AND is_deleted = 0 LIMIT 1",
                )
                .bind(class_name)
                .fetch_optional(pool)
                .await?;
                context.class_id = class_id;
            }
        }

        Ok(context)
    }

    /// 并行搜索多数据源
    async fn parallel_search(
        &self,
        pool: &SqlitePool,
        workspace_path: &Path,
        context: &SearchContext,
        _classification: &IntentClassification,
    ) -> Result<Vec<(EvidenceItem, EvidenceSourceType)>, AppError> {
        let mut searches: Vec<SearchTask> = Vec::new();

        // 学生档案搜索
        if let Some(student_id) = &context.student_id {
            let pool_clone = pool.clone();
            let student_id_clone = student_id.clone();
            searches.push(tokio::spawn(async move {
                Self::search_student_profile(&pool_clone, &student_id_clone).await
            }));
        }

        // 记忆证据搜索
        let keyword = if !context.keywords.is_empty() {
            Some(context.keywords.join(" "))
        } else {
            None
        };

        if keyword.is_some() || context.student_id.is_some() || context.class_id.is_some() {
            let pool_clone = pool.clone();
            let workspace_clone = workspace_path.to_path_buf();
            let search_input = MemorySearchInput {
                keyword,
                student_id: context.student_id.clone(),
                class_id: context.class_id.clone(),
                from_date: context.from_date.clone(),
                to_date: context.to_date.clone(),
                subject: context.subject.clone(),
                source_table: None,
                top_k: Some(10),
                workspace_path: Some(workspace_clone.to_string_lossy().to_string()),
            };

            searches.push(tokio::spawn(async move {
                Self::search_memory_evidence(&pool_clone, &workspace_clone, search_input).await
            }));
        }

        // 等待所有搜索完成
        let mut all_evidence = Vec::new();
        for task in searches {
            match task.await {
                Ok(Ok(evidence)) => all_evidence.extend(evidence),
                Ok(Err(e)) => {
                    eprintln!("[AgenticSearch] 搜索任务失败: {}", e);
                }
                Err(e) => {
                    eprintln!("[AgenticSearch] 搜索任务 panic: {}", e);
                }
            }
        }

        Ok(all_evidence)
    }

    /// 搜索学生档案
    async fn search_student_profile(
        pool: &SqlitePool,
        student_id: &str,
    ) -> Result<Vec<(EvidenceItem, EvidenceSourceType)>, AppError> {
        let profile = StudentService::get_profile_360(pool, student_id).await?;

        let mut evidence = Vec::new();

        // 添加基本信息
        let basic_info = format!(
            "学生姓名: {}, 学号: {}, 性别: {}",
            profile.student.name,
            profile.student.student_no,
            profile.student.gender.as_deref().unwrap_or("未知")
        );

        evidence.push((
            EvidenceItem {
                content: basic_info,
                source_table: String::from("student_profile"),
                source_id: student_id.to_string(),
                student_id: student_id.to_string(),
                class_id: Some(profile.student.class_id.clone()),
                created_at: profile.student.created_at.clone(),
                score: 1.0,
                file_path: None,
                subject: None,
            },
            EvidenceSourceType::StudentProfile,
        ));

        // 添加标签信息
        if !profile.tags.is_empty() {
            let tags_content = format!(
                "学生标签: {}",
                profile
                    .tags
                    .iter()
                    .map(|t| t.tag_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            evidence.push((
                EvidenceItem {
                    content: tags_content,
                    source_table: String::from("student_tags"),
                    source_id: student_id.to_string(),
                    student_id: student_id.to_string(),
                    class_id: Some(profile.student.class_id.clone()),
                    created_at: Utc::now().to_rfc3339(),
                    score: 0.9,
                    file_path: None,
                    subject: None,
                },
                EvidenceSourceType::StudentProfile,
            ));
        }

        // 添加最近观察记录
        for obs in profile.recent_observations.iter().take(3) {
            evidence.push((
                EvidenceItem {
                    content: obs.content.clone(),
                    source_table: String::from("observation_note"),
                    source_id: obs.id.clone(),
                    student_id: student_id.to_string(),
                    class_id: Some(profile.student.class_id.clone()),
                    created_at: obs.created_at.clone(),
                    score: 0.8,
                    file_path: None,
                    subject: None,
                },
                EvidenceSourceType::MemoryEvidence,
            ));
        }

        Ok(evidence)
    }

    /// 搜索记忆证据
    async fn search_memory_evidence(
        pool: &SqlitePool,
        workspace_path: &Path,
        input: MemorySearchInput,
    ) -> Result<Vec<(EvidenceItem, EvidenceSourceType)>, AppError> {
        let result = MemorySearchService::search_evidence(pool, workspace_path, input).await?;

        Ok(result
            .items
            .into_iter()
            .map(|item| (item, EvidenceSourceType::MemoryEvidence))
            .collect())
    }

    /// 证据去重和排序
    fn deduplicate_and_rank(
        &self,
        evidence: Vec<(EvidenceItem, EvidenceSourceType)>,
    ) -> Vec<(EvidenceItem, EvidenceSourceType)> {
        let mut unique: HashMap<String, (EvidenceItem, EvidenceSourceType)> = HashMap::new();

        for (item, source_type) in evidence {
            let key = format!("{}:{}", item.source_table, item.source_id);

            match unique.get(&key) {
                Some((existing, _)) if existing.score >= item.score => {}
                _ => {
                    unique.insert(key, (item, source_type));
                }
            }
        }

        let mut ranked: Vec<(EvidenceItem, EvidenceSourceType)> = unique.into_values().collect();
        ranked.sort_by(|a, b| {
            b.0.score
                .partial_cmp(&a.0.score)
                .expect("证据分数比较失败：存在 NaN 值或不可比较的分数")
        });

        // 限制数量
        ranked.truncate(15);
        ranked
    }

    /// 组装搜索结果
    fn assemble_result(
        &self,
        evidence: Vec<(EvidenceItem, EvidenceSourceType)>,
        classification: &IntentClassification,
    ) -> AgenticSearchResult {
        let sources: Vec<EvidenceSource> = evidence
            .into_iter()
            .map(|(item, source_type)| EvidenceSource::from_evidence_item(&item, source_type))
            .collect();

        let confidence_score = if sources.is_empty() {
            0.0
        } else {
            let avg_score: f64 = sources
                .iter()
                .map(|s| s.relevance_score as f64)
                .sum::<f64>()
                / sources.len() as f64;
            (avg_score * classification.confidence as f64).min(1.0) as f32
        };

        let mut risk_warnings = Vec::new();
        if sources.is_empty() {
            risk_warnings.push(String::from("未找到相关证据，回答可能不够准确"));
        }
        if confidence_score < 0.5 {
            risk_warnings.push(String::from("置信度较低，建议人工核实"));
        }

        AgenticSearchResult {
            conclusion: format!(
                "找到 {} 条相关证据，意图识别: {}",
                sources.len(),
                classification.intent.description()
            ),
            evidence_sources: sources,
            confidence_score,
            risk_warnings,
        }
    }
}

impl Default for AgenticSearchOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplicate_and_rank() {
        let orchestrator = AgenticSearchOrchestrator::new();

        let evidence = vec![
            (
                EvidenceItem {
                    content: String::from("测试内容1"),
                    source_table: String::from("test"),
                    source_id: String::from("1"),
                    student_id: String::from("s1"),
                    class_id: None,
                    created_at: Utc::now().to_rfc3339(),
                    score: 0.8,
                    file_path: None,
                    subject: None,
                },
                EvidenceSourceType::MemoryEvidence,
            ),
            (
                EvidenceItem {
                    content: String::from("测试内容1重复"),
                    source_table: String::from("test"),
                    source_id: String::from("1"),
                    student_id: String::from("s1"),
                    class_id: None,
                    created_at: Utc::now().to_rfc3339(),
                    score: 0.6,
                    file_path: None,
                    subject: None,
                },
                EvidenceSourceType::MemoryEvidence,
            ),
        ];

        let result = orchestrator.deduplicate_and_rank(evidence);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.score, 0.8);
    }
}

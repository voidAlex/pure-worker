//! 记忆检索服务模块
//!
//! 提供 Agentic Search 的统一检索能力：SQL 精确过滤、FTS 全文召回、文件检索与规则重排。

use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, SqlitePool};
use tokio::fs;

use crate::error::AppError;
use crate::models::memory_search::{EvidenceItem, FtsRow, MemorySearchInput, SearchEvidenceResult};

/// 记忆检索服务。
pub struct MemorySearchService;

impl MemorySearchService {
    /// 统一证据检索：组合 SQL 过滤 + FTS 召回 + 文件遍历 + 规则重排 + Top-K 截断。
    pub async fn search_evidence(
        pool: &SqlitePool,
        workspace_path: &Path,
        input: MemorySearchInput,
    ) -> Result<SearchEvidenceResult, AppError> {
        let top_k = input.top_k.unwrap_or(10);
        if top_k <= 0 {
            return Err(AppError::InvalidInput(String::from(
                "top_k 必须是大于 0 的整数",
            )));
        }

        let (sql_items, fts_items, file_items) = tokio::try_join!(
            sql_filter(pool, &input),
            fts_recall(pool, &input),
            file_search(workspace_path, &input)
        )?;

        let total_before_dedup = (sql_items.len() + fts_items.len() + file_items.len()) as i64;

        let mut merged = Vec::with_capacity(total_before_dedup as usize);
        merged.extend(sql_items);
        merged.extend(fts_items);
        merged.extend(file_items);

        let mut dedup_map: HashMap<String, EvidenceItem> = HashMap::new();
        for item in merged {
            match dedup_map.get(&item.source_id) {
                Some(existing) if existing.score >= item.score => {}
                _ => {
                    dedup_map.insert(item.source_id.clone(), item);
                }
            }
        }

        let mut dedup_items: Vec<EvidenceItem> = dedup_map.into_values().collect();
        rule_rerank(&mut dedup_items, &input);

        let mut limited = dedup_items;
        limited.truncate(top_k as usize);

        Ok(SearchEvidenceResult {
            returned_count: limited.len() as i64,
            items: limited,
            total_before_dedup,
        })
    }
}

/// SQL 精确过滤：从 memory_fts 按条件精确查询。
async fn sql_filter(
    pool: &SqlitePool,
    input: &MemorySearchInput,
) -> Result<Vec<EvidenceItem>, AppError> {
    let mut query = QueryBuilder::new(
        "SELECT source_table, source_id, student_id, class_id, content, created_at, 0.0 as rank FROM memory_fts WHERE 1 = 1",
    );

    if let Some(student_id) = input.student_id.as_deref() {
        query.push(" AND student_id = ").push_bind(student_id);
    }

    if let Some(class_id) = input.class_id.as_deref() {
        query.push(" AND class_id = ").push_bind(class_id);
    }

    if let Some(from_date) = input.from_date.as_deref() {
        query.push(" AND created_at >= ").push_bind(from_date);
    }

    if let Some(to_date) = input.to_date.as_deref() {
        query.push(" AND created_at <= ").push_bind(to_date);
    }

    if let Some(source_table) = input.source_table.as_deref() {
        query.push(" AND source_table = ").push_bind(source_table);
    }

    query.push(" ORDER BY created_at DESC");

    let rows = query
        .build_query_as::<FtsRow>()
        .fetch_all(pool)
        .await
        .map_err(|error| AppError::Database(error.to_string()))?;

    let items = rows
        .into_iter()
        .map(|row| EvidenceItem {
            class_id: normalize_optional_text(Some(row.class_id)),
            content: row.content,
            created_at: row.created_at,
            file_path: None,
            score: 0.1,
            source_id: row.source_id,
            source_table: row.source_table,
            student_id: row.student_id,
            subject: None,
        })
        .collect();

    Ok(items)
}

/// FTS 全文检索：使用 SQLite FTS5 MATCH 语法召回。
async fn fts_recall(
    pool: &SqlitePool,
    input: &MemorySearchInput,
) -> Result<Vec<EvidenceItem>, AppError> {
    let Some(keyword) = input.keyword.as_deref() else {
        return Ok(Vec::new());
    };

    let trimmed = keyword.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = QueryBuilder::new(
        "SELECT source_table, source_id, student_id, class_id, content, created_at, rank FROM memory_fts WHERE memory_fts MATCH ",
    );
    query.push_bind(trimmed);

    if let Some(student_id) = input.student_id.as_deref() {
        query.push(" AND student_id = ").push_bind(student_id);
    }

    if let Some(class_id) = input.class_id.as_deref() {
        query.push(" AND class_id = ").push_bind(class_id);
    }

    if let Some(from_date) = input.from_date.as_deref() {
        query.push(" AND created_at >= ").push_bind(from_date);
    }

    if let Some(to_date) = input.to_date.as_deref() {
        query.push(" AND created_at <= ").push_bind(to_date);
    }

    if let Some(source_table) = input.source_table.as_deref() {
        query.push(" AND source_table = ").push_bind(source_table);
    }

    query.push(" ORDER BY rank");

    let rows = query
        .build_query_as::<FtsRow>()
        .fetch_all(pool)
        .await
        .map_err(|error| AppError::Database(error.to_string()))?;

    let items = rows
        .into_iter()
        .map(|row| EvidenceItem {
            class_id: normalize_optional_text(Some(row.class_id)),
            content: row.content,
            created_at: row.created_at,
            file_path: None,
            score: row.rank,
            source_id: row.source_id,
            source_table: row.source_table,
            student_id: row.student_id,
            subject: None,
        })
        .collect();

    Ok(items)
}

/// 文件遍历：在学生 memory 目录搜索 Markdown 文件内容。
async fn file_search(
    workspace_path: &Path,
    input: &MemorySearchInput,
) -> Result<Vec<EvidenceItem>, AppError> {
    let Some(keyword) = input.keyword.as_deref() else {
        return Ok(Vec::new());
    };

    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }

    if let Some(source_table) = input.source_table.as_deref() {
        if source_table != "file" {
            return Ok(Vec::new());
        }
    }

    let students_root = workspace_path.join("students");
    let mut target_student_ids = Vec::new();

    if let Some(student_id) = input.student_id.as_deref() {
        target_student_ids.push(student_id.to_string());
    } else {
        let mut dir = match fs::read_dir(&students_root).await {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(error) => {
                return Err(AppError::FileOperation(format!(
                    "读取学生目录失败：{}",
                    error
                )));
            }
        };

        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|error| AppError::FileOperation(format!("遍历学生目录失败：{}", error)))?
        {
            let file_type = entry.file_type().await.map_err(|error| {
                AppError::FileOperation(format!("读取目录项类型失败：{}", error))
            })?;
            if file_type.is_dir() {
                target_student_ids.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }

    let mut items = Vec::new();

    for student_id in target_student_ids {
        let memory_dir = students_root.join(&student_id).join("memory");
        let mut memory_entries = match fs::read_dir(&memory_dir).await {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(AppError::FileOperation(format!(
                    "读取记忆目录失败：{}",
                    error
                )));
            }
        };

        while let Some(entry) = memory_entries
            .next_entry()
            .await
            .map_err(|error| AppError::FileOperation(format!("遍历记忆目录失败：{}", error)))?
        {
            let file_type = entry
                .file_type()
                .await
                .map_err(|error| AppError::FileOperation(format!("读取文件类型失败：{}", error)))?;
            if !file_type.is_file() {
                continue;
            }

            let path = entry.path();
            let is_markdown = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("md"))
                .unwrap_or(false);
            if !is_markdown {
                continue;
            }

            let content = fs::read_to_string(&path)
                .await
                .map_err(|error| AppError::FileOperation(format!("读取记忆文件失败：{}", error)))?;

            let metadata = fs::metadata(&path).await.map_err(|error| {
                AppError::FileOperation(format!("读取记忆文件元数据失败：{}", error))
            })?;
            let created_at = metadata
                .modified()
                .map(DateTime::<Utc>::from)
                .map(|time| time.to_rfc3339())
                .unwrap_or_else(|_| Utc::now().to_rfc3339());

            for (index, line) in content.lines().enumerate() {
                if line.contains(keyword) {
                    let file_path = path.to_string_lossy().to_string();
                    items.push(EvidenceItem {
                        class_id: input.class_id.clone(),
                        content: line.to_string(),
                        created_at: created_at.clone(),
                        file_path: Some(file_path.clone()),
                        score: 0.15,
                        source_id: format!("{}:{}", file_path, index + 1),
                        source_table: String::from("file"),
                        student_id: student_id.clone(),
                        subject: input.subject.clone(),
                    });
                }
            }
        }
    }

    Ok(items)
}

/// 规则重排：对合并的证据列表按规则计算最终分数。
fn rule_rerank(items: &mut [EvidenceItem], input: &MemorySearchInput) {
    let now = Utc::now();

    let fts_ranks: Vec<f64> = items
        .iter()
        .filter(|item| item.source_table != "file" && item.score != 0.1)
        .map(|item| item.score)
        .collect();

    let (min_rank, max_rank) = if fts_ranks.is_empty() {
        (0.0_f64, 0.0_f64)
    } else {
        let min_rank =
            fts_ranks.iter().fold(
                f64::INFINITY,
                |acc, value| if *value < acc { *value } else { acc },
            );
        let max_rank =
            fts_ranks.iter().fold(
                f64::NEG_INFINITY,
                |acc, value| if *value > acc { *value } else { acc },
            );
        (min_rank, max_rank)
    };

    for item in items.iter_mut() {
        let mut score = 0.0_f64;

        if let Ok(created_at) = DateTime::parse_from_rfc3339(&item.created_at) {
            let age_days = (now - created_at.with_timezone(&Utc)).num_days();
            if age_days <= 7 {
                score += 0.5;
            } else if age_days <= 30 {
                score += 0.3;
            }
        }

        if let (Some(input_subject), Some(item_subject)) =
            (input.subject.as_deref(), item.subject.as_deref())
        {
            if input_subject == item_subject {
                score += 0.2;
            }
        }

        if item.source_table != "file" && item.score != 0.1 {
            let normalized = if (max_rank - min_rank).abs() < f64::EPSILON {
                1.0
            } else {
                (max_rank - item.score) / (max_rank - min_rank)
            };
            score += normalized.clamp(0.0, 1.0);
        } else {
            score += item.score;
        }

        item.score = score.clamp(0.0, 1.0);
    }

    items.sort_by(|left, right| right.score.total_cmp(&left.score));
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    match value {
        Some(text) if text.trim().is_empty() => None,
        other => other,
    }
}

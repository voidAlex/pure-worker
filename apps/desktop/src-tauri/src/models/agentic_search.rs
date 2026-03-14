//! Agentic Search 结果模型
//!
//! 定义 Agentic Search 的输入输出数据结构。

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::memory_search::EvidenceItem;

/// 证据来源类型
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum EvidenceSourceType {
    /// 学生档案
    StudentProfile,
    /// 记忆证据
    MemoryEvidence,
    /// 课堂记录
    LessonRecord,
    /// 作业记录
    AssignmentRecord,
    /// 家校沟通
    CommunicationHistory,
}

impl EvidenceSourceType {
    /// 获取来源类型的中文描述
    pub fn description(&self) -> &'static str {
        match self {
            EvidenceSourceType::StudentProfile => "学生档案",
            EvidenceSourceType::MemoryEvidence => "记忆证据",
            EvidenceSourceType::LessonRecord => "课堂记录",
            EvidenceSourceType::AssignmentRecord => "作业记录",
            EvidenceSourceType::CommunicationHistory => "家校沟通",
        }
    }
}

/// 证据来源
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EvidenceSource {
    /// 来源类型
    pub source_type: EvidenceSourceType,
    /// 来源ID
    pub source_id: String,
    /// 证据内容摘要
    pub content_summary: String,
    /// 完整内容
    pub full_content: String,
    /// 相关度评分
    pub relevance_score: f32,
    /// 创建时间
    pub created_at: Option<String>,
    /// 额外元数据
    pub metadata: Option<serde_json::Value>,
}

/// Agentic Search 结果
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgenticSearchResult {
    /// 结论摘要
    pub conclusion: String,
    /// 证据来源列表
    pub evidence_sources: Vec<EvidenceSource>,
    /// 置信度评分 (0.0 ~ 1.0)
    pub confidence_score: f32,
    /// 风险提示
    pub risk_warnings: Vec<String>,
}

/// Agentic Search 输入
#[derive(Debug, Deserialize, Type)]
pub struct AgenticSearchInput {
    /// 用户原始查询
    pub query: String,
    /// 会话ID（用于缓存）
    pub session_id: Option<String>,
    /// 是否强制刷新缓存
    pub force_refresh: Option<bool>,
}

/// 搜索上下文
#[derive(Debug, Clone)]
pub struct SearchContext {
    /// 学生ID（已解析）
    pub student_id: Option<String>,
    /// 班级ID（已解析）
    pub class_id: Option<String>,
    /// 学科
    pub subject: Option<String>,
    /// 日期范围起始
    pub from_date: Option<String>,
    /// 日期范围截止
    pub to_date: Option<String>,
    /// 关键词
    pub keywords: Vec<String>,
}

/// 搜索结果缓存项
#[derive(Debug, Clone)]
pub struct SearchCacheEntry {
    /// 查询内容哈希
    pub query_hash: String,
    /// 搜索结果
    pub result: AgenticSearchResult,
    /// 缓存时间
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

impl AgenticSearchResult {
    /// 创建空结果
    pub fn empty() -> Self {
        Self {
            conclusion: String::from("未找到相关证据"),
            evidence_sources: Vec::new(),
            confidence_score: 0.0,
            risk_warnings: Vec::new(),
        }
    }

    /// 格式化证据为上下文字符串
    pub fn format_evidence_context(&self) -> String {
        let mut context = String::new();

        for (index, source) in self.evidence_sources.iter().enumerate() {
            context.push_str(&format!(
                "[证据{}] {} (相关度: {:.2}):\n{}\n\n",
                index + 1,
                source.source_type.description(),
                source.relevance_score,
                source.full_content
            ));
        }

        context
    }
}

impl EvidenceSource {
    /// 从 EvidenceItem 创建
    pub fn from_evidence_item(item: &EvidenceItem, source_type: EvidenceSourceType) -> Self {
        Self {
            source_type,
            source_id: item.source_id.clone(),
            content_summary: item.content.chars().take(100).collect::<String>() + "...",
            full_content: item.content.clone(),
            relevance_score: item.score as f32,
            created_at: Some(item.created_at.clone()),
            metadata: None,
        }
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

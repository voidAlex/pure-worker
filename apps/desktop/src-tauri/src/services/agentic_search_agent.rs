//! Agentic Search Rig Agent
//!
//! 使用 Rig 框架构建的 Agent，通过工具调用执行多源检索。

use rig::completion::ToolDefinition;
use rig::tool::ToolDyn;
use serde_json::json;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;

use crate::error::AppError;
use crate::models::agentic_search::{AgenticSearchInput, AgenticSearchResult};
use crate::services::agentic_search::AgenticSearchOrchestrator;
use crate::services::memory_search::MemorySearchService;
use crate::services::student::StudentService;

/// Agentic Search Agent 构建器
pub struct AgenticSearchAgentBuilder {
    orchestrator: Arc<AgenticSearchOrchestrator>,
}

impl AgenticSearchAgentBuilder {
    /// 创建新的 Agent 构建器
    pub fn new() -> Self {
        Self {
            orchestrator: Arc::new(AgenticSearchOrchestrator::new()),
        }
    }

    /// 构建工具集
    pub fn build_tools(
        &self,
        pool: SqlitePool,
        workspace_path: std::path::PathBuf,
    ) -> Vec<Box<dyn ToolDyn>> {
        vec![
            Box::new(SearchStudentTool { pool: pool.clone() }),
            Box::new(SearchMemoryTool {
                pool,
                workspace_path,
            }),
        ]
    }

    /// 执行搜索（非 Agent 模式）
    pub async fn execute_search(
        &self,
        pool: &SqlitePool,
        workspace_path: &Path,
        query: &str,
    ) -> Result<AgenticSearchResult, AppError> {
        self.orchestrator
            .search(
                pool,
                workspace_path,
                AgenticSearchInput {
                    query: query.to_string(),
                    session_id: None,
                    force_refresh: None,
                },
            )
            .await
    }
}

impl Default for AgenticSearchAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 搜索学生工具
#[derive(Debug)]
struct SearchStudentTool {
    pool: SqlitePool,
}

impl ToolDyn for SearchStudentTool {
    fn name(&self) -> String {
        String::from("search_student")
    }

    fn definition<'a>(
        &'a self,
        _prompt: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolDefinition> + Send + 'a>> {
        let def = ToolDefinition {
            name: String::from("search_student"),
            description: String::from("根据学生姓名或ID搜索学生基本信息和档案"),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name_or_id": {
                        "type": "string",
                        "description": "学生姓名或学生ID"
                    }
                },
                "required": ["name_or_id"]
            }),
        };
        Box::pin(async move { def })
    }

    fn call<'a>(
        &'a self,
        args: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, rig::tool::ToolError>> + Send + 'a>,
    > {
        let pool = self.pool.clone();

        Box::pin(async move {
            let input: serde_json::Value =
                serde_json::from_str(&args).map_err(rig::tool::ToolError::JsonError)?;

            let name_or_id = input
                .get("name_or_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            // 先尝试作为 ID 查找
            let student_result = StudentService::get_by_id(&pool, name_or_id).await;

            let student = match student_result {
                Ok(s) => s,
                Err(_) => {
                    // 尝试作为姓名查找
                    let student_id: Option<String> = sqlx::query_scalar(
                        "SELECT id FROM student WHERE name = ? AND is_deleted = 0 LIMIT 1",
                    )
                    .bind(name_or_id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|e| {
                        rig::tool::ToolError::ToolCallError(Box::new(ToolAdapterError(
                            e.to_string(),
                        )))
                    })?;

                    match student_id {
                        Some(id) => StudentService::get_by_id(&pool, &id).await.map_err(|e| {
                            rig::tool::ToolError::ToolCallError(Box::new(ToolAdapterError(
                                e.to_string(),
                            )))
                        })?,
                        None => {
                            return Ok(json!({
                                "found": false,
                                "error": format!("未找到学生: {}", name_or_id)
                            })
                            .to_string());
                        }
                    }
                }
            };

            // 获取 360 档案
            let profile = StudentService::get_profile_360(&pool, &student.id)
                .await
                .map_err(|e| {
                    rig::tool::ToolError::ToolCallError(Box::new(ToolAdapterError(e.to_string())))
                })?;

            let result = json!({
                "found": true,
                "student": {
                    "id": student.id,
                    "name": student.name,
                    "student_no": student.student_no,
                    "gender": student.gender,
                    "class_id": student.class_id,
                    "tags": profile.tags.iter().map(|t| &t.tag_name).collect::<Vec<_>>(),
                    "recent_observations_count": profile.recent_observations.len(),
                    "recent_scores_count": profile.recent_scores.len(),
                    "recent_communications_count": profile.recent_communications.len(),
                }
            });

            serde_json::to_string(&result).map_err(rig::tool::ToolError::JsonError)
        })
    }
}

/// 搜索记忆工具
#[derive(Debug)]
struct SearchMemoryTool {
    pool: SqlitePool,
    workspace_path: std::path::PathBuf,
}

impl ToolDyn for SearchMemoryTool {
    fn name(&self) -> String {
        String::from("search_memory")
    }

    fn definition<'a>(
        &'a self,
        _prompt: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolDefinition> + Send + 'a>> {
        let def = ToolDefinition {
            name: String::from("search_memory"),
            description: String::from("搜索学生记忆证据，包括观察记录、沟通记录、评语等"),
            parameters: json!({
                "type": "object",
                "properties": {
                    "keyword": {
                        "type": "string",
                        "description": "搜索关键词"
                    },
                    "student_id": {
                        "type": "string",
                        "description": "学生ID（可选）"
                    },
                    "class_id": {
                        "type": "string",
                        "description": "班级ID（可选）"
                    },
                    "subject": {
                        "type": "string",
                        "description": "学科（可选）"
                    }
                },
                "required": ["keyword"]
            }),
        };
        Box::pin(async move { def })
    }

    fn call<'a>(
        &'a self,
        args: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, rig::tool::ToolError>> + Send + 'a>,
    > {
        let pool = self.pool.clone();
        let workspace_path = self.workspace_path.clone();

        Box::pin(async move {
            let input: serde_json::Value =
                serde_json::from_str(&args).map_err(rig::tool::ToolError::JsonError)?;

            let keyword = input
                .get("keyword")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let student_id = input
                .get("student_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            let class_id = input
                .get("class_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            let subject = input
                .get("subject")
                .and_then(|v| v.as_str())
                .map(String::from);

            let search_input = crate::models::memory_search::MemorySearchInput {
                keyword: Some(keyword),
                student_id,
                class_id,
                from_date: None,
                to_date: None,
                subject,
                source_table: None,
                top_k: Some(10),
                workspace_path: Some(workspace_path.to_string_lossy().to_string()),
            };

            let result = MemorySearchService::search_evidence(&pool, &workspace_path, search_input)
                .await
                .map_err(|e| {
                    rig::tool::ToolError::ToolCallError(Box::new(ToolAdapterError(e.to_string())))
                })?;

            let evidence: Vec<serde_json::Value> = result
                .items
                .into_iter()
                .map(|item| {
                    json!({
                        "content": item.content,
                        "source": item.source_table,
                        "source_id": item.source_id,
                        "created_at": item.created_at,
                        "score": item.score,
                    })
                })
                .collect();

            let result = json!({
                "total": evidence.len(),
                "evidence": evidence
            });

            serde_json::to_string(&result).map_err(rig::tool::ToolError::JsonError)
        })
    }
}

/// 适配器内部错误类型
#[derive(Debug)]
struct ToolAdapterError(String);

impl std::fmt::Display for ToolAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolAdapterError {}

/// Agentic Search 工具集封装
pub struct AgenticSearchToolSet {
    tools: Vec<Box<dyn ToolDyn>>,
}

impl AgenticSearchToolSet {
    /// 创建工具集
    pub fn new(pool: SqlitePool, workspace_path: std::path::PathBuf) -> Self {
        let builder = AgenticSearchAgentBuilder::new();
        Self {
            tools: builder.build_tools(pool, workspace_path),
        }
    }

    /// 获取工具列表
    pub fn tools(&self) -> &[Box<dyn ToolDyn>] {
        &self.tools
    }

    /// 转换为 vec（用于 Agent 创建）
    pub fn into_vec(self) -> Vec<Box<dyn ToolDyn>> {
        self.tools
    }
}

/// 格式化搜索结果为提示词上下文
pub fn format_search_result_for_prompt(result: &AgenticSearchResult) -> String {
    let mut context = String::from("## 相关证据\n\n");

    if result.evidence_sources.is_empty() {
        context.push_str("未找到相关证据。\n");
    } else {
        for (index, source) in result.evidence_sources.iter().enumerate() {
            context.push_str(&format!(
                "{}. 【{}】{}\n\n",
                index + 1,
                source.source_type.description(),
                source.full_content
            ));
        }
    }

    if !result.risk_warnings.is_empty() {
        context.push_str("\n## 风险提示\n\n");
        for warning in &result.risk_warnings {
            context.push_str(&format!("- {}\n", warning));
        }
    }

    context
}

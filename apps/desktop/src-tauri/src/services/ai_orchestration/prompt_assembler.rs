//! 提示词运行时组装服务
//!
//! 负责把 profile、模板、证据、工具摘要与用户输入按统一层级拼接成最终提示词。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::AppError;
use crate::models::execution::ExecutionRequest;
use crate::services::ai_orchestration::agent_profile_registry::OutputProtocol;
use crate::services::prompt_template::PromptTemplateService;
use crate::services::prompt_template_registry::{
    load_multimodal_template, render_content_items, runtime_template_name, ContentItem,
};

use super::{
    model_routing::SelectedModel, OrchestrationError, OrchestrationResult, PromptAssembler,
    RuntimeAgentProfile,
};

/// 组装后的提示词
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledPrompt {
    pub system_prompt: String,
    pub user_prompt: String,
    pub template_name: Option<String>,
    pub used_multimodal_template: bool,
}

/// PromptAssembler 具体实现
pub struct PromptAssemblerService {
    templates_dir: PathBuf,
}

impl PromptAssemblerService {
    pub fn new(templates_dir: impl AsRef<Path>) -> Self {
        Self {
            templates_dir: templates_dir.as_ref().to_path_buf(),
        }
    }

    pub fn assemble(
        &self,
        request: &ExecutionRequest,
        profile: &RuntimeAgentProfile,
        selected_model: &SelectedModel,
        evidence: &[String],
        tool_summary: &str,
    ) -> OrchestrationResult<AssembledPrompt> {
        let is_multimodal_request = !request.attachments.is_empty();
        if is_multimodal_request && !selected_model.capability.supports_image_input {
            return Err(OrchestrationError::ModelCapabilityInsufficient(
                String::from("当前请求包含附件，但选中模型不支持多模态输入"),
            ));
        }

        let template_name = runtime_template_name(&profile.id, is_multimodal_request)
            .map(std::string::ToString::to_string);
        let used_multimodal_template = is_multimodal_request
            && template_name
                .as_deref()
                .map(|name| name.contains("multimodal"))
                .unwrap_or(false);

        let variables = build_variables(request, evidence, tool_summary);
        let task_layer = self.render_task_layer(template_name.as_deref(), &variables)?;

        let system_prompt = join_sections(&[
            String::from("[system]\n你是 PureWorker 的统一 AI Runtime 执行器。"),
            format!(
                "[profile]\nProfile: {}\n说明: {}\n入口: {:?}",
                profile.name, profile.description, profile.entrypoint
            ),
            task_layer.system,
        ]);

        let user_prompt = join_sections(&[
            format!(
                "[evidence]\n{}",
                join_lines_or_placeholder(evidence, "无检索证据")
            ),
            format!(
                "[tool summary]\n{}",
                if tool_summary.trim().is_empty() {
                    "无工具摘要"
                } else {
                    tool_summary
                }
            ),
            format!(
                "[output protocol]\n{}",
                output_protocol_text(profile.output_protocol)
            ),
            task_layer.user,
            format!("[user input]\n{}", request.user_input),
        ]);

        Ok(AssembledPrompt {
            system_prompt,
            user_prompt,
            template_name,
            used_multimodal_template,
        })
    }

    fn render_task_layer(
        &self,
        template_name: Option<&str>,
        variables: &HashMap<String, String>,
    ) -> OrchestrationResult<TaskLayer> {
        let Some(template_name) = template_name else {
            return Ok(TaskLayer::default());
        };

        let template_path = self
            .templates_dir
            .join("templates")
            .join(format!("{template_name}.toml"));

        if template_name.contains("multimodal") {
            let template = load_multimodal_template(&template_path).map_err(map_app_error)?;
            let system = render_content_items(&template.template.system, variables)
                .map_err(map_app_error)?;
            let user =
                render_content_items(&template.template.user, variables).map_err(map_app_error)?;
            return Ok(TaskLayer {
                system: format!("[task]\n{}", content_items_to_text(&system)),
                user: format!("[task user]\n{}", content_items_to_text(&user)),
            });
        }

        let template = PromptTemplateService::load_template(&self.templates_dir, template_name)
            .map_err(map_app_error)?;
        let rendered =
            PromptTemplateService::render(&template, variables).map_err(map_app_error)?;

        Ok(TaskLayer {
            system: format!("[task]\n{}", rendered.system),
            user: format!("[task user]\n{}", rendered.user),
        })
    }
}

impl PromptAssembler for PromptAssemblerService {
    fn assemble(
        &self,
        request: &ExecutionRequest,
        profile: &RuntimeAgentProfile,
        selected_model: &SelectedModel,
        evidence: &[String],
        tool_summary: &str,
    ) -> OrchestrationResult<AssembledPrompt> {
        PromptAssemblerService::assemble(
            self,
            request,
            profile,
            selected_model,
            evidence,
            tool_summary,
        )
    }
}

#[derive(Default)]
struct TaskLayer {
    system: String,
    user: String,
}

fn build_variables(
    request: &ExecutionRequest,
    evidence: &[String],
    tool_summary: &str,
) -> HashMap<String, String> {
    let mut variables = HashMap::new();

    if let Some(Value::Object(object)) = &request.metadata_json {
        for (key, value) in object {
            variables.insert(key.clone(), json_value_to_string(value));
        }
    }

    variables.insert(String::from("user_input"), request.user_input.clone());
    variables
        .entry(String::from("query"))
        .or_insert_with(|| request.user_input.clone());
    variables.insert(
        String::from("evidence_text"),
        join_lines_or_placeholder(evidence, "无检索证据"),
    );
    variables.insert(
        String::from("tool_summary"),
        if tool_summary.trim().is_empty() {
            String::from("无工具摘要")
        } else {
            tool_summary.to_string()
        },
    );

    let image_descriptions = request
        .attachments
        .iter()
        .map(|attachment| {
            attachment
                .display_name
                .clone()
                .unwrap_or_else(|| attachment.path.clone())
        })
        .collect::<Vec<String>>()
        .join("；");
    if !image_descriptions.is_empty() {
        variables.insert(String::from("image_descriptions"), image_descriptions);
    }

    variables
}

fn json_value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::Array(items) => items
            .iter()
            .map(json_value_to_string)
            .collect::<Vec<String>>()
            .join("，"),
        Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn output_protocol_text(output_protocol: OutputProtocol) -> &'static str {
    match output_protocol {
        OutputProtocol::Markdown => "输出 Markdown 草稿内容。",
        OutputProtocol::Json => "输出严格 JSON 结构。",
    }
}

fn content_items_to_text(items: &[ContentItem]) -> String {
    items
        .iter()
        .map(|item| match item {
            ContentItem::Text { content } => content.clone(),
            ContentItem::Image { url, detail } => {
                if let Some(detail) = detail {
                    format!("[image] {} ({})", url, detail)
                } else {
                    format!("[image] {}", url)
                }
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn join_sections(sections: &[String]) -> String {
    sections
        .iter()
        .filter(|section| !section.trim().is_empty())
        .cloned()
        .collect::<Vec<String>>()
        .join("\n\n")
}

fn join_lines_or_placeholder(lines: &[String], placeholder: &str) -> String {
    if lines.is_empty() {
        placeholder.to_string()
    } else {
        lines.join("\n")
    }
}

fn map_app_error(error: AppError) -> OrchestrationError {
    match error {
        AppError::InvalidInput(message) => OrchestrationError::InvalidRequest(message),
        AppError::NotFound(message) => OrchestrationError::Internal(message),
        AppError::FileOperation(message) | AppError::Config(message) => {
            OrchestrationError::Internal(message)
        }
        AppError::Database(message) => OrchestrationError::Store(message),
        AppError::TaskExecution(message) => OrchestrationError::Internal(message),
        AppError::PermissionDenied(message) => OrchestrationError::Internal(message),
        AppError::ExternalService(message) => OrchestrationError::ProviderUnavailable(message),
        AppError::Internal(message) => OrchestrationError::Internal(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ai_config::ModelCapability;
    use crate::models::execution::{
        ExecutionAttachment, ExecutionEntrypoint, ExecutionRequest, StreamMode,
    };
    use crate::services::ai_orchestration::{
        agent_profile_registry::AgentProfileRegistry, AgentProfileResolver,
    };
    use serde_json::json;

    fn templates_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("packages")
            .join("prompt-templates")
    }

    fn selected_model(
        model_id: &str,
        supports_image_input: bool,
        supports_json_mode: bool,
    ) -> SelectedModel {
        SelectedModel {
            provider_id: String::from("cfg-openai"),
            model_id: model_id.to_string(),
            capability: ModelCapability {
                supports_text_input: true,
                supports_image_input,
                supports_audio_input: false,
                supports_tool_calling: true,
                supports_reasoning: false,
                supports_json_mode,
                context_window: 128000,
                max_output_tokens: 8192,
            },
            fallback_used: false,
            trace: crate::services::ai_orchestration::model_routing::RoutingTrace {
                requested_capability: String::from("text"),
                candidate_model: model_id.to_string(),
                selected_model: model_id.to_string(),
                fallback_chain: vec![model_id.to_string()],
            },
        }
    }

    fn homeroom_request(attachments: Vec<ExecutionAttachment>) -> ExecutionRequest {
        ExecutionRequest {
            session_id: None,
            entrypoint: ExecutionEntrypoint::Chat,
            agent_profile_id: String::from("chat.homeroom"),
            user_input: String::from("请帮我回复这位家长"),
            attachments,
            use_agentic_search: false,
            stream_mode: StreamMode::Streaming,
            metadata_json: Some(json!({
                "student_name": "张三",
                "query": "孩子最近上课状态怎么样？",
                "class_info": "三年级二班",
                "student_tags": "积极、热心"
            })),
        }
    }

    /// 验证文本请求使用 text 模板并保留工具摘要
    #[test]
    fn test_homeroom_text_template_and_tool_summary() {
        let assembler = PromptAssemblerService::new(templates_dir());
        let registry = AgentProfileRegistry::new_default();
        let profile = AgentProfileResolver::get_profile(&registry, "chat.homeroom").unwrap();
        let request = homeroom_request(vec![]);

        let result = assembler.assemble(
            &request,
            &profile,
            &selected_model("gpt-4o-mini", false, false),
            &[String::from("证据A"), String::from("证据B")],
            "已执行 search.student",
        );

        let assembled = result.expect("text assembly should succeed");
        assert_eq!(
            assembled.template_name.as_deref(),
            Some("chat_homeroom_text")
        );
        assert!(!assembled.used_multimodal_template);
        assert!(assembled.user_prompt.contains("已执行 search.student"));
        assert!(assembled.system_prompt.contains("班主任对话"));
    }

    /// 验证多模态请求使用 multimodal 模板
    #[test]
    fn test_homeroom_multimodal_template_when_attachment_exists() {
        let assembler = PromptAssemblerService::new(templates_dir());
        let registry = AgentProfileRegistry::new_default();
        let profile = AgentProfileResolver::get_profile(&registry, "chat.homeroom").unwrap();
        let request = homeroom_request(vec![ExecutionAttachment {
            path: String::from("/tmp/photo.png"),
            media_type: Some(String::from("image/png")),
            display_name: Some(String::from("课堂照片")),
        }]);

        let result = assembler.assemble(
            &request,
            &profile,
            &selected_model("gpt-4o", true, true),
            &[String::from("图像证据")],
            "",
        );

        let assembled = result.expect("multimodal assembly should succeed");
        assert_eq!(
            assembled.template_name.as_deref(),
            Some("chat_homeroom_multimodal")
        );
        assert!(assembled.used_multimodal_template);
        assert!(assembled.system_prompt.contains("课堂照片"));
    }

    /// 验证 grading 走多模态 JSON 模板
    #[test]
    fn test_grading_template_selected() {
        let assembler = PromptAssemblerService::new(templates_dir());
        let registry = AgentProfileRegistry::new_default();
        let profile = AgentProfileResolver::get_profile(&registry, "chat.grading").unwrap();
        let request = ExecutionRequest {
            session_id: None,
            entrypoint: ExecutionEntrypoint::Grading,
            agent_profile_id: String::from("chat.grading"),
            user_input: String::from("请批改这份作业"),
            attachments: vec![ExecutionAttachment {
                path: String::from("/tmp/homework.jpg"),
                media_type: Some(String::from("image/jpeg")),
                display_name: Some(String::from("作业图片")),
            }],
            use_agentic_search: false,
            stream_mode: StreamMode::Streaming,
            metadata_json: Some(json!({
                "assignment_type": "课堂练习",
                "grading_criteria": "按步骤给分",
                "subject": "数学"
            })),
        };

        let result = assembler.assemble(
            &request,
            &profile,
            &selected_model("gpt-4o", true, true),
            &[],
            "",
        );

        let assembled = result.expect("grading assembly should succeed");
        assert_eq!(
            assembled.template_name.as_deref(),
            Some("grading_multimodal_json")
        );
        assert!(assembled.system_prompt.contains("JSON"));
    }

    /// 验证 search-enabled profile 会注入证据层
    #[test]
    fn test_search_enabled_profile_includes_evidence_layer() {
        let assembler = PromptAssemblerService::new(templates_dir());
        let registry = AgentProfileRegistry::new_default();
        let profile = AgentProfileResolver::get_profile(&registry, "search.agentic").unwrap();
        let request = ExecutionRequest {
            session_id: None,
            entrypoint: ExecutionEntrypoint::Search,
            agent_profile_id: String::from("search.agentic"),
            user_input: String::from("帮我汇总这名学生的近期表现"),
            attachments: vec![],
            use_agentic_search: true,
            stream_mode: StreamMode::Streaming,
            metadata_json: None,
        };

        let result = assembler.assemble(
            &request,
            &profile,
            &selected_model("gpt-4o-mini", false, false),
            &[String::from("观察记录1"), String::from("观察记录2")],
            "",
        );

        let assembled = result.expect("search assembly should succeed");
        assert!(assembled.user_prompt.contains("观察记录1"));
        assert!(assembled.user_prompt.contains("观察记录2"));
        assert!(assembled.template_name.is_none());
    }

    /// 验证多模态请求在模型能力不足时显式失败
    #[test]
    fn test_multimodal_request_without_capable_model_fails() {
        let assembler = PromptAssemblerService::new(templates_dir());
        let registry = AgentProfileRegistry::new_default();
        let profile = AgentProfileResolver::get_profile(&registry, "chat.homeroom").unwrap();
        let request = homeroom_request(vec![ExecutionAttachment {
            path: String::from("/tmp/photo.png"),
            media_type: Some(String::from("image/png")),
            display_name: Some(String::from("课堂照片")),
        }]);

        let result = assembler.assemble(
            &request,
            &profile,
            &selected_model("gpt-4o-mini", false, false),
            &[],
            "",
        );

        assert!(matches!(
            result,
            Err(OrchestrationError::ModelCapabilityInsufficient(_))
        ));
    }
}

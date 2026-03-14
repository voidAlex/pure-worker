# WP-AI-008: 多模态 Prompt 模板体系实现计划

## 目标
建立支持多模态、版本化、能力分型的 Prompt 模板体系，支持按模型能力和任务场景自动选择模板。

## 现状
- 已有基础模板结构（meta + template）
- 仅支持文本模板
- 无模型能力匹配机制
- 多模态 prompt 仍硬编码在代码中

## 实现内容

### 1. 扩展模板元数据结构
新增字段：
- `task_type`: chat / communication / grading / agentic_search_summary
- `modality`: text / multimodal
- `capability_requirements`: ["json_mode", "tool_calling", "vision"]
- `output_protocol`: markdown / structured_json / draft_card
- `model_capability_hint`: 建议的模型能力标签
- `fallback_template`: 降级模板名称

### 2. 创建多模态 Prompt 模板示例
- `chat_homeroom_text.toml`
- `chat_homeroom_multimodal.toml`
- `grading_multimodal_json.toml`
- `student_analysis_agentic_search.toml`

### 3. 扩展 Rust 服务
- `PromptTemplateRegistry`: 模板注册与查询
- `TemplateSelector`: 根据模型能力和任务选择模板
- 支持模板降级策略

### 4. 数据库支持
- `prompt_template_registry` 表：记录可用模板和元数据
- 支持模板启用/禁用

## 文件变更
- `src/services/prompt_template.rs`: 扩展模板结构和服务
- `src/models/prompt_template.rs`: 新增模型定义
- `migrations/0011_prompt_templates.sql`: 数据库迁移
- `packages/prompt-templates/templates/*.toml`: 新增模板文件

## 验收标准
- [ ] 模板支持多模态标记
- [ ] 支持按模型能力自动选择模板
- [ ] 支持模板降级策略
- [ ] 所有多模态场景有对应模板

# PureWorker AI 能力缺口审核报告

> 审核时间：2026-03-14
> 审核依据：`doc/ai-capability-gap-action-plan.md`
> 审核方式：代码库深度扫描 + 迁移文件检查 + 服务实现验证

---

## 执行摘要

经过全面审核，当前代码库已完成大部分 AI 能力缺口补齐工作，整体进度约 **75%**。

### 总体状态

| 主线 | 进度 | 状态 |
|------|------|------|
| 主线 A：工作台会话化 | 80% | 🟡 部分完成 |
| 主线 B：模型与供应商运行时 | 85% | 🟡 部分完成 |
| 主线 C：Agent 执行链 | 70% | 🟡 部分完成 |
| 主线 D：教师场景知识底座 | 90% | ✅ 基本完成 |

---

## 详细任务审核

### ✅ 已完成任务

#### WP-AI-001：工作台会话持久化

**状态：✅ 后端完成，🟡 前端部分完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| conversation 表 is_deleted | ✅ | 0002_add_soft_delete_to_conversation.sql |
| conversation_message 表 is_deleted | ✅ | 0002_add_soft_delete_to_conversation.sql |
| ConversationService | ✅ | services/conversation_service.rs |
| list_conversations IPC | ✅ | commands/chat.rs:471-481 |
| get_chat_conversation IPC | ✅ | commands/chat.rs:484-491 |
| delete_chat_conversation IPC | ✅ | commands/chat.rs:494-501 |
| chat_stream 流式命令 | ✅ | commands/chat.rs:152-242 |
| ChatStreamEvent 事件 | ✅ | models/conversation.rs |

**待完成：**
- 前端 AiPanel 需要接入历史列表 UI
- AiPanel 需要切换到 ChatMessage 组件使用 Markdown 渲染

---

#### WP-AI-004：Provider Adapter 重构

**状态：✅ 已完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| ProviderType 枚举 | ✅ | services/provider_adapter/mod.rs:21-47 |
| OpenAI 兼容适配器 | ✅ | services/provider_adapter/openai_adapter.rs |
| Anthropic 原生适配器 | ✅ | services/provider_adapter/anthropic_adapter.rs |
| AdapterFactory | ✅ | services/provider_adapter/mod.rs:60-89 |
| 供应商预设 | ✅ | services/llm_provider.rs:559-587 |

---

#### WP-AI-009：行课记录模型

**状态：✅ 已完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| lesson_record 表 | ✅ | 0012_lesson_record.sql:18-35 |
| schedule_event_id 外键 | ✅ | 0012_lesson_record.sql:21 |
| 业务表 lesson_record_id | ✅ | 0012_lesson_record.sql:42-54 |
| 索引 | ✅ | 0012_lesson_record.sql:61-85 |

---

#### WP-AI-010：Agentic Search 编排器

**状态：✅ 已完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| AgenticSearchOrchestrator | ✅ | services/agentic_search.rs:30-428 |
| 意图分类 | ✅ | services/intent_classifier.rs |
| 实体解析 | ✅ | services/agentic_search.rs:143-194 |
| 并行搜索 | ✅ | services/agentic_search.rs:197-257 |
| 证据去重排序 | ✅ | services/agentic_search.rs:356-379 |
| 证据链回答 | ✅ | services/agentic_search.rs:382-421 |
| 工作台集成 | ✅ | commands/chat.rs:263-344 |

---

#### WP-AI-006：统一 Tool Registry

**状态：✅ 已完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| ToolCategory 枚举 | ✅ | services/tool_registry.rs:12-20 |
| UnifiedTool trait | ✅ | services/unified_tool.rs:15-46 |
| ToolRegistry | ✅ | services/tool_registry.rs:35-188 |
| Builtin 工具注册 | ✅ | services/tool_registry.rs:builtin_tools() |
| Skill 工具适配 | ✅ | services/skill_tool_adapter.rs |
| MCP 工具适配 | ✅ | services/mcp_tool_adapter.rs |
| 角色白名单 | ✅ | services/tool_registry.rs:103-134 |

---

#### WP-AI-011：教师偏好记忆

**状态：✅ 基本完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| teacher_preference 表 | ✅ | 0013_teacher_memory.sql:15-26 |
| memory_candidate 表 | ✅ | 0013_teacher_memory.sql:44-58 |
| TeacherMemoryService | ✅ | services/teacher_memory.rs |
| 偏好 CRUD | ✅ | services/teacher_memory.rs:55-145 |
| 候选记忆管理 | ✅ | services/teacher_memory.rs:148-228 |
| 模式检测 | ✅ | services/teacher_memory.rs:231-287 |
| 默认偏好数据 | ✅ | 0013_teacher_memory.sql:97-116 |

**待完成：**
- soul.md / user.md 文件存储（当前仅预留字段）
- Session Memory 层（可选）

---

#### WP-AI-008：多模态 Prompt 模板

**状态：✅ 已完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| 模板文件（6个） | ✅ | packages/prompt-templates/templates/ |
| Modality 枚举 | ✅ | services/prompt_template_registry.rs:40-54 |
| ModelCapability 枚举 | ✅ | services/prompt_template_registry.rs:76-96 |
| TemplateSelector | ✅ | services/prompt_template_registry.rs:176-277 |
| 能力匹配 | ✅ | services/prompt_template_registry.rs:match_template() |
| Fallback 链 | ✅ | services/prompt_template_registry.rs:select_with_fallback() |
| prompt_template_registry 表 | ✅ | 0011_prompt_template_registry.sql |

**注意：**
- multimodal_grading.rs 仍存在硬编码 prompt（第77-99行）
- 建议后续迁移到模板系统

---

### 🟡 部分完成任务

#### WP-AI-002：Markdown 渲染与消息卡片

**状态：🟡 部分完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| react-markdown 依赖 | ✅ | package.json:25 |
| remark-gfm 依赖 | ✅ | package.json:27 |
| ChatMessage 组件 | ✅ | components/chat/ChatMessage.tsx |
| AiPanel 接入 | ❌ | AiPanel.tsx:265-271 仍用 whitespace-pre-wrap |

**问题：**
AiPanel.tsx 直接渲染消息文本，未使用 ChatMessage 组件的 Markdown 渲染能力。

**建议修复：**
将 AiPanel.tsx 第 254-278 行替换为 ChatMessage 组件调用。

---

#### WP-AI-003：聊天流式事件

**状态：🟡 部分完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| ChatStreamEvent 定义 | ✅ | models/conversation.rs |
| chat_stream IPC 命令 | ✅ | commands/chat.rs:152-242 |
| Start / Chunk / Complete 事件 | ✅ | commands/chat.rs:207-228 |
| ThinkingStatus 事件 | ✅ | commands/chat.rs:267-397 |
| SearchSummary 事件 | ✅ | commands/chat.rs:298-305 |
| Reasoning 事件 | ✅ | commands/chat.rs:308-313 |
| 真实流式生成 | ❌ | 当前使用模拟流式（按句子分割） |

**问题：**
第 358-368 行使用非流式生成，第 442-460 行用句子分割模拟流式。

**代码注释明确说明：**
```rust
// 使用非流式方式生成（简化实现）
// 实际流式实现需要使用 ProviderAdapter 的 chat_stream 方法
```

---

#### WP-AI-005：模型能力元数据与多模型配置

**状态：🟡 部分完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| ModelCapability 结构体 | ✅ | models/ai_config.rs:63-89 |
| supports_text_input | ✅ | bool 字段 |
| supports_image_input | ✅ | bool 字段 |
| supports_tool_calling | ✅ | bool 字段 |
| supports_reasoning | ✅ | bool 字段 |
| supports_json_mode | ✅ | bool 字段 |
| context_window | ✅ | u32 字段 |
| max_output_tokens | ✅ | u32 字段 |
| MultiModelConfig 结构体 | ✅ | models/ai_config.rs:104-115 |
| 数据库字段拆分 | ❌ | 0005_ai_config.sql 仅 default_model |
| 配置使用拆分字段 | ❌ | llm_provider.rs 使用单一 default_model |

**问题：**
虽然数据结构已定义，但数据库和运行时仍使用单一的 `default_model`。

---

#### WP-AI-007：MCP Runtime 接入

**状态：🟡 部分完成**

| 检查项 | 状态 | 代码位置 |
|--------|------|----------|
| McpClient 实现 | ✅ | services/mcp_runtime.rs:1-328 |
| tools/list | ✅ | services/mcp_runtime.rs:list_tools() |
| tools/call | ✅ | services/mcp_runtime.rs:call_tool() |
| McpToolAdapter | ✅ | services/mcp_tool_adapter.rs |
| 注册到 Tool Registry | ⚠️ | 提供 register_mcp_tools() 但未在启动时调用 |
| Agent 执行链使用 | ❌ | agentic_search_agent.rs 仍用硬编码工具 |

**问题：**
1. `register_mcp_tools` 未在应用启动时调用（lib.rs 未初始化）
2. Agent 执行链未使用 Tool Registry，仍使用硬编码工具

---

### ❌ 未完成任务

#### WP-AI-001 前端：会话列表 UI

**状态：❌ 未完成**

- 后端 IPC 命令已就绪：list_conversations, get_chat_conversation, delete_chat_conversation
- 前端 AiPanel 未集成历史列表 UI
- 需要添加：
  - 左侧会话列表侧边栏
  - 会话切换功能
  - 新建会话按钮
  - 删除会话功能

---

## 需要确认的问题

### 问题 1：前端 AiPanel 的 Markdown 渲染集成方式

**当前状态：**
- ChatMessage 组件已实现 Markdown 渲染
- AiPanel 仍使用纯文本渲染

**选项：**
1. **完全替换**：将 AiPanel 的消息渲染部分完全替换为 ChatMessage 组件
2. **渐进升级**：保持 AiPanel 现状，仅在新的聊天界面使用 ChatMessage
3. **AiPanel 改造**：改造 AiPanel 支持 ChatMessage 组件，但保留原有布局

**建议：** 选项 3 - 改造 AiPanel 使用 ChatMessage 组件，工作量小且用户体验一致。

---

### 问题 2：流式生成的实现程度

**当前状态：**
- 使用句子级模拟流式
- TODO 注释说明需要实现真正的 token 级流式

**选项：**
1. **保持现状**：句子级模拟流式已能满足基本需求
2. **实现真正流式**：需要扩展 ProviderAdapter 支持 chat_stream 方法

**建议：** 选项 1 - 当前实现已满足"思考状态展示"的需求，真正流式可作为后续优化。

---

### 问题 3：多模型配置的数据库迁移

**当前状态：**
- MultiModelConfig 结构体已定义
- 数据库仍使用单一 default_model 字段

**选项：**
1. **保持现状**：单模型配置在 MVP 阶段足够
2. **立即迁移**：拆分为 default_text_model / default_multimodal_model 字段

**建议：** 选项 2 - 建议立即迁移，因为涉及数据库 schema 变更，越早变更成本越低。

---

### 问题 4：Tool Registry 与 Agent 执行链的集成

**当前状态：**
- Tool Registry 统一建模已完成
- Agent 执行链仍使用硬编码工具

**选项：**
1. **保持现状**：硬编码工具在当前 Agent 场景下工作正常
2. **部分集成**：Agentic Search Agent 使用 Tool Registry，其他保持现状
3. **完全集成**：所有 Agent 统一使用 Tool Registry

**建议：** 选项 2 - Agentic Search Agent 使用 Tool Registry 即可体现架构价值，其他场景可后续逐步迁移。

---

### 问题 5：multimodal_grading.rs 的硬编码 Prompt

**当前状态：**
- 第 77-99 行为硬编码 system_prompt 和 user_prompt
- Prompt 模板系统已实现

**选项：**
1. **保持现状**：硬编码 prompt 在当前场景下工作正常
2. **迁移到模板**：将 prompt 提取到 TOML 模板文件

**建议：** 选项 2 - 迁移工作量小，符合架构设计，建议立即执行。

---

## 推荐执行顺序

### 立即执行（P0）

1. **修复 AiPanel Markdown 渲染**
   - 改造 AiPanel 使用 ChatMessage 组件
   - 工作量：小（1-2 小时）

2. **数据库迁移：多模型配置**
   - 添加 default_text_model / default_multimodal_model 字段
   - 工作量：中（2-3 小时）

3. **Agentic Search Agent 使用 Tool Registry**
   - 替换硬编码工具为 Tool Registry 查询
   - 工作量：中（3-4 小时）

### 近期执行（P1）

4. **multimodal_grading.rs 迁移到模板**
   - 创建 grading_multimodal.toml 模板
   - 工作量：小（1 小时）

5. **MCP 注册初始化**
   - 在 lib.rs 应用启动时调用 register_mcp_tools
   - 工作量：小（30 分钟）

6. **AiPanel 历史列表 UI**
   - 添加左侧会话列表
   - 工作量：中（4-5 小时）

### 后续优化（P2）

7. **实现真正流式生成**
   - 扩展 ProviderAdapter 支持 chat_stream
   - 工作量：大（1-2 天）

8. **soul.md / user.md 文件存储**
   - 实现文件读写和解析
   - 工作量：中（3-4 小时）

---

## 结论

当前 AI 能力缺口补齐工作已完成约 **75%**，核心架构（Provider Adapter、Tool Registry、Agentic Search、行课记录、偏好记忆）均已落地。

剩余工作主要是：
1. **前端集成**（AiPanel Markdown + 历史列表）
2. **数据库字段拆分**（多模型配置）
3. **架构对齐**（Tool Registry 接入 Agent）

建议优先完成 P0 任务，可使工作台达到基本可用状态。

---

*报告生成时间：2026-03-14*
*报告生成人：Sisyphus AI*

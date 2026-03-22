# PureWorker Agent Runtime / Orchestration Layer 设计方案

> 设计时间：2026-03-21  
> 设计范围：较大范围  
> 状态：已完成方案评审，可进入实现计划阶段  
> 前置调研：`doc/agent-runtime-2026-03-21/open-source-agent-architecture-research-2026-03-21.md`

---

## 1. 目标

为 pure-worker 设计一层统一的 **AI Orchestration Layer**，位于现有 IPC 与底层 Provider / Prompt / Skill / MCP 能力之间，用来解决当前“零件已存在，但执行链分散”的问题。

本次设计覆盖：

1. Agent Runtime 核心抽象
2. Prompt Assembler
3. Tool Exposure
4. Session Event
5. Provider Runtime 升级入口
6. 多 agent 扩展预留点

本次设计**包含**彻底重构以下范围：

1. 统一 AI 执行入口，替换现有分散入口
2. 重建 Prompt Runtime，而不是在旧模板链上继续叠加
3. 重建 Session Event 协议与执行存储模型
4. 把 Agentic Search 内聚到统一 Orchestration Layer

本次设计**不包含**：

1. ACP / 外部 control-plane 接入
2. openclaw 风格 gateway 网络控制面

---

## 2. 设计结论

本方案采用 **B：中台式编排层** 作为主方案。

核心判断：

1. 继续沿着现有 `chat.rs + llm_provider + prompt_template_registry + skill_tool_adapter + mcp_runtime` 做局部增强，只会继续延长“局部可用、整体不协调”的状态。
2. 当前需要的不是包裹旧执行链，而是直接建立新的唯一执行主链。
3. 最合适的路径，是把现有 Provider、Skills、MCP 等能力降为新 Runtime 的底层依赖，同时删除旧入口、旧拼装路径和旧事件路径。

---

## 3. 备选方案对比

### 3.1 方案 A：渐进式内核

特点：

- 只在现有 `chat.rs`、`agentic_search`、`llm_provider` 周边加内核抽象
- 改动小，落地快

优点：

- 风险最低
- 对现有代码扰动小

缺点：

- 很容易演变成“新旧逻辑并存但边界不清”
- 难以真正统一 grading / communication / search / chat

### 3.2 方案 B：中台式编排层（推荐）

特点：

- 新增统一 Orchestration Layer
- 所有 AI 入口统一收口成 `ExecutionRequest -> ExecutionOrchestrator`
- 旧 IPC 命令、旧 Prompt 拼装链、旧 Agentic Search 外围编排全部删除或重写

优点：

- 可以真正统一执行链
- 有利于后续多模型、会话事件、工具暴露、多 agent 扩展
- 不会留下双轨兼容债务

缺点：

- 需要引入一组新的核心抽象
- 首次重构成本高于渐进迁移

### 3.3 方案 C：双层并存架构

特点：

- 底层是 runtime core
- 上层保留多个旧入口并长期共存

优点：

- 对旧逻辑最友好

缺点：

- 如果没有明确淘汰路径，容易变成长期双轨制
- 架构清晰度不如方案 B

### 3.4 推荐理由

选择 **方案 B**，因为它最接近调研报告要求的统一 Agent Runtime 主链：

- 比 A 更能解决结构性问题；
- 比 C 更彻底，不会留下兼容层；
- 同时又不引入 openclaw 那类当前阶段不匹配的 control-plane。

---

## 4. 总体架构

目标结构如下：

```text
Tauri IPC Commands
  -> ExecutionRequestFactory
    -> ExecutionOrchestrator
      -> AgentProfileRegistry
      -> PromptAssembler
      -> ToolExposureService
      -> ModelRoutingService
      -> ProviderAdapter / Rig Agent Runtime
      -> SessionEventBus
      -> ExecutionStore
```

### 4.1 设计原则

1. **统一入口**：所有 AI 执行请求最终都进入 `ExecutionOrchestrator`。
2. **分层装配**：Prompt、Model、Tools 不在 IPC 层拼装。
3. **事件优先**：执行过程一律转成统一 Session Event。
4. **会话级视图**：工具暴露不再只看全局 registry，还要看当前会话、角色、风险级别。
5. **单轨重构**：新 Runtime 是唯一执行主链，旧命令与旧拼装链不保留。
6. **边界清晰**：不引入重量级 control-plane，但在新 Runtime 内直接定义可扩展接口。

---

## 5. 核心组件设计

### 5.1 ExecutionOrchestrator

职责：

1. 接收统一 `ExecutionRequest`
2. 读取 `AgentProfile`
3. 协调 Prompt、Model、Tools 的装配
4. 执行模型调用或工具调用链
5. 发出统一事件
6. 在结束后写入结果与摘要

它是整个方案的中心，不直接关心 UI，不直接关心 Tauri IPC 细节。

### 5.2 AgentProfileRegistry

职责：

1. 定义角色级别的执行策略
2. 给出该角色可用的：
   - prompt policy
   - model policy
   - tool exposure policy
   - output protocol
   - verification policy
3. 为未来 subagent 定义 profile 类型

建议 profile 至少覆盖：

- `chat.homeroom`
- `chat.grading`
- `chat.communication`
- `chat.ops`
- `search.agentic`

后续再扩展：

- `subagent.explore`
- `subagent.summarize`
- `subagent.verify`

建议 Profile 结构：

```rust
pub struct AgentProfile {
    pub id: String,
    pub display_name: String,
    pub entrypoint: ExecutionEntrypoint,
    pub system_prompt_key: String,
    pub template_task_type: Option<String>,
    pub output_protocol: OutputProtocol,
    pub capability_requirements: Vec<ModelCapabilityKey>,
    pub tool_allowlist: Vec<String>,
    pub tool_denylist: Vec<String>,
    pub max_tool_risk: ToolRiskLevel,
    pub requires_search: bool,
}
```

加载策略：

1. `AgentProfile` 作为 Runtime 一等配置对象落到独立持久化层；
2. 运行时启动时加载完整 profile 集，不再依赖硬编码分支；
3. profile 变更以数据更新驱动，而不是代码 if/else 驱动。

### 5.3 PromptAssembler

职责：

1. 组合系统层 prompt、角色层 prompt、业务模板层 prompt
2. 根据模型能力与输出协议生成最终 prompt
3. 插入工具摘要、证据上下文、教师审阅闭环要求

建议分四层：

1. **System Layer**：本地优先、人审闭环、安全边界
2. **Profile Layer**：班主任/批改/沟通/检索角色约束
3. **Task Layer**：业务模板、任务上下文、证据摘要
4. **Execution Layer**：工具列表摘要、输出协议、事件提示

### 5.4 ToolExposureService

职责：

1. 从 builtin / skill / MCP 聚合工具候选集
2. 基于 `AgentProfile + SessionContext + 风险级别 + 用户配置` 过滤
3. 输出“当前会话可见工具视图”
4. 为前端或日志提供工具清单摘要

它不替代 `ToolRegistry`，而是建立在 `ToolRegistry` 之上的会话级曝光层。

### 5.5 ModelRoutingService

职责：

1. 读取 `AiConfig`、`ProviderRegistry`、`ModelCatalog` 和 `ModelInfo`
2. 基于任务类型 / profile / capability 选择模型
3. 在 `text_only / multimodal / tool / reasoning` 四类执行路径之间路由
4. 对齐 opencode 的 provider runtime 思路：同一运行时内支持多供应商差异化 loader、模型发现和 fallback
5. 输出可审计的路由摘要（选模原因、是否 fallback、能力匹配结果）

本设计直接定义完整的 Provider Runtime 范围：

- capability router
- model selection policy
- provider fallback hook
- model catalog
- provider capability metadata
- provider registry（供应商注册与启停状态）
- provider loader matrix（按供应商协议类型路由调用链）
- discovery + static catalog 双轨模型清单

明确不纳入的仅有：

- ACP
- 外部 gateway control-plane

### 5.6 SessionEventBus

职责：

1. 统一发出执行过程事件
2. 对接统一的 Runtime 事件分发通道
3. 让非流式与流式入口都共享相同事件语义

建议标准事件集：

- `Start`
- `ThinkingStatus`
- `ToolCall`
- `ToolResult`
- `SearchSummary`
- `Reasoning`
- `Chunk`
- `Complete`
- `Error`
- `ExecutionSummary`（新增）

### 5.7 ExecutionStore

职责：

1. 持久化会话与消息
2. 保存执行级元数据与事件摘要
3. 保留失败摘要，避免流式中断后无痕丢失
4. 为后续执行回放、调试、审计提供基础数据

建议不要把所有原始 token/event 全量永久存表；执行存储以结构化执行记录和关键轨迹为主。

依赖注入建议：

```rust
pub struct AiOrchestrationRuntime {
    pub profiles: Arc<dyn AgentProfileRegistry + Send + Sync>,
    pub prompt_assembler: Arc<dyn PromptAssembler + Send + Sync>,
    pub tool_exposure: Arc<dyn ToolExposureService + Send + Sync>,
    pub model_routing: Arc<dyn ModelRoutingService + Send + Sync>,
    pub event_bus: Arc<dyn SessionEventBus + Send + Sync>,
    pub store: Arc<dyn ExecutionStore + Send + Sync>,
}
```

原则：

1. `ExecutionOrchestrator` 仅依赖 trait，不直接依赖具体实现；
2. 在 Tauri `setup` 时完成实例装配；
3. 测试中可替换为 mock/fake 实现。

---

## 6. 核心数据对象

### 6.1 ExecutionRequest

统一请求对象，建议字段：

- `session_id: Option<String>`
- `entrypoint: ExecutionEntrypoint`（chat / grading / communication / search）
- `agent_profile_id: String`
- `user_input: String`
- `attachments: Vec<ExecutionAttachment>`
- `use_agentic_search: bool`
- `stream_mode: StreamMode`（streaming / non_streaming）
- `metadata_json: Option<Value>`

字段约束：

| 字段 | 是否必填 | 约束 |
|------|----------|------|
| `session_id` | 否 | 流式续聊时必填；新会话可为空 |
| `entrypoint` | 是 | 仅允许 `chat / grading / communication / search` |
| `agent_profile_id` | 是 | 必须能在 `AgentProfileRegistry` 中解析 |
| `user_input` | 是 | trim 后不能为空 |
| `attachments` | 否 | 首期仅允许本地文件句柄/路径元数据，不传文件正文 |
| `use_agentic_search` | 否 | 默认 false |
| `stream_mode` | 是 | `streaming` 或 `non_streaming` |
| `metadata_json` | 否 | 仅允许对象类型，禁止数组和原始标量 |

建议 Rust 结构：

```rust
pub enum ExecutionEntrypoint {
    Chat,
    Grading,
    Communication,
    Search,
}

pub enum StreamMode {
    Streaming,
    NonStreaming,
}

pub struct ExecutionAttachment {
    pub path: String,
    pub media_type: Option<String>,
    pub display_name: Option<String>,
}

pub struct ExecutionRequest {
    pub session_id: Option<String>,
    pub entrypoint: ExecutionEntrypoint,
    pub agent_profile_id: String,
    pub user_input: String,
    pub attachments: Vec<ExecutionAttachment>,
    pub use_agentic_search: bool,
    pub stream_mode: StreamMode,
    pub metadata_json: Option<serde_json::Value>,
}
```

### 6.2 SessionContext

建议聚合：

- 当前会话信息
- 最近历史消息
- 当前教师身份
- 当前工作区路径
- 当前可见工具集
- 当前模型策略

建议 Rust 结构：

```rust
pub struct SessionContext {
    pub conversation: Option<Conversation>,
    pub recent_messages: Vec<ConversationMessage>,
    pub teacher_id: String,
    pub workspace_path: std::path::PathBuf,
    pub visible_tools: Vec<SessionToolView>,
    pub selected_model: SelectedModel,
}
```

### 6.3 ExecutionPlan

由 orchestrator 在真正执行前生成，包含：

- 使用哪个 profile
- 使用哪个 model
- 用哪些 prompt layers
- 暴露哪些 tools
- 是否启用 search
- 是否启用 reasoning mode

建议 Rust 结构：

```rust
pub struct ExecutionPlan {
    pub profile: AgentProfile,
    pub selected_model: SelectedModel,
    pub assembled_prompt: String,
    pub visible_tools: Vec<SessionToolView>,
    pub enable_search: bool,
    pub enable_reasoning_summary: bool,
}
```

生成算法（首期规则）：

```text
1. 校验 ExecutionRequest
2. 读取 AgentProfile
3. 读取 SessionContext（历史消息、教师、工作区）
4. 按 AgentProfile 的 output_protocol / capability_requirements 请求 ModelRoutingService 选模
5. 按 AgentProfile + SessionContext 计算 SessionToolView
6. 如果 request.use_agentic_search=true 或 profile.requires_search=true，则标记 enable_search
7. 将 system/profile/template/evidence/tool summary 交给 PromptAssembler 生成最终 prompt
8. 得到 ExecutionPlan
```

### 6.4 ExecutionResult

建议包含：

- `final_text`
- `used_model`
- `tool_calls_summary`
- `search_summary`
- `reasoning_summary`
- `status`
- `error_message`

建议 Rust 结构：

```rust
pub enum ExecutionStatus {
    Completed,
    Failed,
    Cancelled,
}

pub struct ExecutionResult {
    pub final_text: String,
    pub used_model: String,
    pub tool_calls_summary: Vec<ToolCallSummary>,
    pub search_summary: Option<SearchSummary>,
    pub reasoning_summary: Option<String>,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
}
```

### 6.5 SessionEvent 协议

为避免前后端协议漂移，首期引入显式版本字段。

```rust
pub const SESSION_EVENT_VERSION: u32 = 1;

pub enum SessionEvent {
    Start { version: u32, message_id: String },
    ThinkingStatus { version: u32, stage: String, description: String },
    ToolCall { version: u32, tool_name: String, input: serde_json::Value },
    ToolResult { version: u32, tool_name: String, output: String, success: bool },
    SearchSummary { version: u32, sources: Vec<String>, evidence_count: usize },
    Reasoning { version: u32, summary: String },
    Chunk { version: u32, content: String },
    ExecutionSummary { version: u32, status: String, used_model: String },
    Complete { version: u32 },
    Error { version: u32, message: String },
}
```

约束：

1. 新 Runtime 直接以 `SessionEvent v1` 作为唯一协议；
2. 前后端同时切换到新协议，不为旧事件模型保留兼容分支；
3. 存储层仅提炼摘要，不存全部 chunk。

### 6.6 ExecutionStore 存储策略

采用 **重构后的会话模型 + `execution_record` 执行表**，不以旧 `conversation` / `conversation_message` 结构为设计约束。

原因：

1. 旧会话模型无法完整表达执行语义与事件摘要；
2. 执行维度数据天然独立于消息正文；
3. 新模型更利于回放、统计、失败追踪与后续多 agent 扩展。

建议新增表：

```text
execution_record
- id
- session_id
- assistant_message_id
- entrypoint
- agent_profile_id
- model_id
- status
- reasoning_summary
- search_summary_json
- tool_calls_summary_json
- error_message
- metadata_json
- created_at
- updated_at
```

关系：

- `execution_session` 1 -> N `execution_message`
- `execution_message(assistant)` 1 -> 0/1 `execution_record`

不新增 event 明细表，避免把事件流变成新的日志垃圾场。

---

## 7. 主数据流

### 7.1 非流式执行入口

```text
execution command
  -> ExecutionRequestFactory
  -> ExecutionOrchestrator
  -> 生成 ExecutionPlan
  -> 调用 ProviderAdapter / Rig Agent
  -> 内部记录事件
  -> 聚合成最终响应
  -> 返回 ExecutionResponse
```

意义：

- 非流式与流式共享同一执行链；
- 不再存在旧 API 外观保留约束。

### 7.2 流式执行入口

```text
execution stream command
  -> ExecutionRequestFactory
  -> ExecutionStore 创建 execution session / message 占位
  -> ExecutionOrchestrator
  -> SessionEventBus 实时发事件
  -> ExecutionStore 更新 execution message / 事件摘要
  -> 前端按统一事件渲染卡片
```

意义：

- `ChatStreamEvent` 被新 `SessionEvent` 统一替换；
- 前端卡片渲染改为只消费新事件协议。

### 7.3 Agentic Search 的接入方式

Agentic Search 不再保留外围专用 orchestrator，而是直接内聚到统一执行链中：

1. 检索作为 `ExecutionStage` 的标准阶段之一；
2. 是否执行 search 由 `AgentProfile` 与 `ExecutionPlan` 决定；
3. 输出统一的 `SearchSummary` 与 `Reasoning` 事件。

建议定义 stage 接口，避免“可插拔”只停留在描述层：

```rust
pub trait ExecutionStage {
    async fn run(
        &self,
        request: &ExecutionRequest,
        session: &SessionContext,
    ) -> Result<Option<SearchSummary>, OrchestrationError>;
}
```

首批实现包含 `AgenticSearchStage`，并删除旧 search 外围编排路径。

### 7.4 核心接口草案

为避免实现阶段靠猜，首期接口建议如下：

```rust
pub trait ExecutionOrchestrator {
    async fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Result<ExecutionResult, OrchestrationError>;
}

pub trait AgentProfileRegistry {
    fn get_profile(&self, profile_id: &str) -> Result<AgentProfile, OrchestrationError>;
}

pub trait PromptAssembler {
    fn assemble(
        &self,
        profile: &AgentProfile,
        request: &ExecutionRequest,
        session: &SessionContext,
        evidence: Option<&SearchSummary>,
    ) -> Result<String, OrchestrationError>;
}

pub trait ToolExposureService {
    fn build_session_tools(
        &self,
        profile: &AgentProfile,
        request: &ExecutionRequest,
        session: &SessionContext,
    ) -> Result<Vec<SessionToolView>, OrchestrationError>;
}

pub trait ModelRoutingService {
    fn select_model(
        &self,
        profile: &AgentProfile,
        request: &ExecutionRequest,
    ) -> Result<SelectedModel, OrchestrationError>;
}

pub trait SessionEventBus {
    async fn emit(&self, event: SessionEvent) -> Result<(), OrchestrationError>;
}

pub trait ExecutionStore {
    async fn create_execution_session(
        &self,
        request: &ExecutionRequest,
    ) -> Result<String, OrchestrationError>;

    async fn create_execution_message(
        &self,
        session_id: &str,
    ) -> Result<String, OrchestrationError>;

    async fn finalize_execution(
        &self,
        execution_message_id: &str,
        result: &ExecutionResult,
    ) -> Result<(), OrchestrationError>;

    async fn record_failure(
        &self,
        execution_message_id: &str,
        error: &OrchestrationError,
    ) -> Result<(), OrchestrationError>;
}
```

---

## 8. Prompt 设计

### 8.1 重构 Prompt Runtime

本方案不再把现有 `prompt_template_registry.rs` 视为长期保留前提，而是将其能力重组进新的 Prompt Runtime：

1. 业务模板从“独立注册表”下沉为 `PromptAssembler` 的 task layer 资源；
2. `TaskType / Modality / CapabilityRequirements` 保留其语义，但不保留旧装配路径；
3. 旧模板选择链、fallback 链和分散系统 prompt 拼装逻辑统一移除。

原因：

1. 调研报告已经明确，现有体系是业务模板系统，不是 runtime prompt system；
2. 如果继续保留旧模板运行链，新 Runtime 会再次沦为包裹层而非唯一执行主链。

### 8.2 新增 Prompt Assembler

新增后，prompt 生成过程变为：

```text
System Constraints
 + Agent Profile Prompt
 + Business Template Prompt
 + Search / Evidence Context
 + Tool Summary
 + Output Protocol
 = Final Assembled Prompt
```

### 8.3 输出协议

建议在 AgentProfile 中显式声明输出协议：

- markdown
- structured_json
- draft_card
- search_answer

这样 PromptAssembler 才能稳定地告诉模型“应该怎样输出”，而不是散落在多个调用点里。

### 8.4 Prompt 组装规则

首期采用固定顺序拼装，避免自由组合导致不可测：

```text
1. System Layer
2. Profile Layer
3. Task Layer
4. Evidence Layer（若有）
5. Tool Summary Layer（若有）
6. Output Protocol Layer
7. User Input
```

规则：

1. 每层之间用双换行分隔；
2. Tool Summary 仅暴露名称、用途、输入摘要，不暴露实现细节；
3. Evidence Layer 仅注入结构化摘要，不直接拼接全部原始搜索结果；
4. Output Protocol Layer 必须位于用户输入之前，保证模型最后读到明确输出要求。

示例：

```text
[System Constraints]
你是 PureWorker 本地优先教师助手，所有输出默认为草稿，需教师确认后生效。

[Profile]
你当前角色是班主任助手，优先关注学生管理、家校沟通与证据充分性。

[Task Template]
请基于教师问题生成可执行建议，默认中文输出。

[Evidence Summary]
已检索到 3 条相关证据，来源：学生档案、观察记录、沟通记录。

[Tool Summary]
可用工具：search.student（查学生基础信息）、search.memory（查证据）。

[Output Protocol]
请输出 Markdown，先给结论，再给依据，最后给建议动作。

[User Input]
张三最近状态怎么样？
```

---

## 9. Tool Exposure 设计

### 9.1 当前问题

当前已有：

- `ToolRegistry`
- `build_all_enabled_skill_tools`
- `McpToolAdapter`
- 按角色的白名单

但问题是：

1. 更偏全局视角；
2. 缺少会话级裁剪；
3. 缺少统一的工具视图摘要与事件输出；
4. 无法自然表达“这个 profile 在本会话只能看到哪些工具”。

### 9.2 新设计

建议增加三层：

1. **ToolRegistry**：全局工具源，不变
2. **ToolExposurePolicy**：角色与风险策略
3. **SessionToolView**：当前会话最终可见工具集

### 9.3 ToolExposurePolicy 可参考维度

- AgentProfile
- ToolCategory（builtin / skill / mcp）
- ToolRiskLevel
- 用户设置
- 场景（chat / grading / communication / search）
- 会话是否启用 search / attachments / reasoning

首期过滤逻辑明确为：

```text
visible = registry_tools
  -> filter by profile allowlist / denylist
  -> filter by risk ceiling
  -> filter by session capability need
  -> filter by current entrypoint
  -> filter by enabled status / health status
```

优先级：

1. denylist 最高
2. risk ceiling 第二
3. entrypoint / capability 第三
4. enabled/health 作为最终硬过滤

示例策略：

1. `chat.homeroom`
   - 可见：低风险 builtin + enabled skill + healthy MCP（只读优先）
   - 禁止：高风险文件系统写入型 MCP
2. `search.agentic`
   - 仅可见：`search.student`、`search.memory` 及未来检索类只读工具
3. `chat.grading`
   - 若无附件，则隐藏图像预处理类工具

### 9.4 对 skills / MCP 的要求

首期要做到：

1. Skill 与 MCP 工具都能转为统一 SessionToolView
2. ToolCall / ToolResult 事件使用统一格式
3. MCP 可见性由会话与角色共同决定，而不是仅启动即全局可见

---

## 10. Provider Runtime 升级设计

### 10.1 Provider Runtime 重构范围

本方案直接定义 Provider Runtime 的完整重构范围：

1. `ModelRoutingService`
2. `Capability Router`
3. `Model Catalog`
4. `Fallback Hook`
5. `Provider Registry`
6. `Provider Loader Matrix`
7. `Discovery + Static Catalog Merge`

对齐口径：

1. 本项目供应商接入在运行时抽象层面全面对齐 opencode；
2. 对齐的是 provider runtime 机制（分层、路由、fallback、能力元数据），不是照搬外部 UI 或控制面；
3. 供应商能力必须通过统一 catalog 暴露给 `ModelRoutingService`，禁止在命令层散落 provider 分支。

### 10.2 明确排除的内容

1. ACP
2. 外部 control-plane
3. 重量级 gateway bridge
4. 大规模 provider plugin platform

### 10.3 建议的内部结构

```text
AiConfig / ProviderConfig / ModelInfo
  -> ProviderRegistry
  -> ProviderLoaderMatrix
  -> ModelCatalog(discovery + static fallback)
  -> CapabilityResolver
  -> ModalityPolicyResolver(text_only vs multimodal)
  -> ModelRoutingPolicy
  -> SelectedModel + RoutingTrace
```

这样后续要接 discovery / remote catalog / provider-specific fallback 时，不需要再改 IPC 层。

`ModelCatalog` 首期最小元数据字段要求：

- `provider_id`
- `model_id`
- `input_modalities`（至少区分 text / image）
- `supports_text_input`
- `supports_image_input`
- `supports_tool_calling`
- `supports_reasoning`
- `context_window`
- `max_output_tokens`
- `cost_tier`（用于后续成本路由）

### 10.4 路由决策规则

按以下顺序选模：

```text
1. AgentProfile 明确指定模型 -> 直接使用
2. 从 request + attachments 推导输入模态需求（text_only 或 multimodal）
3. 按 request.entrypoint 映射任务类型（chat/grading/communication/search）
4. 按 capability_requirements 选择 default_text/default_vision/default_tool/default_reasoning
5. 用 ModelCatalog 校验候选模型能力是否满足需求
6. 若为 multimodal 需求，必须命中 supports_image_input=true 的模型，禁止回落到 text-only 模型
7. 若为 text_only 需求，默认优先 text-only 模型；仅在明确配置允许时才可回落 multimodal
8. 若模型能力不足，返回显式错误（`ModelCapabilityInsufficient`），不静默降级
9. provider 不可用时触发 provider 级 fallback hook，并记录路由摘要
```

说明：

1. 优先稳定和可解释，不做复杂打分路由；
2. `default_model` 只用于同能力类别内的显式 fallback，不跨越 text_only/multimodal 能力边界；
3. provider 不可用时才触发 provider 级 fallback hook，且必须记录事件摘要；
4. 对所有 multimodal 请求，路由失败必须在错误摘要中明确指出缺失能力字段。

---

## 11. Session Event 设计

### 11.1 为什么要统一

当前 `chat_stream` 已经有：

- `ThinkingStatus`
- `ToolCall`
- `ToolResult`
- `SearchSummary`
- `Reasoning`
- `Chunk`
- `Complete`
- `Error`

这是好基础，但目前仍偏 chat 命令本地逻辑，不是 runtime 公共协议。

### 11.2 新方案

将其提升为运行时公共事件协议：

- 所有 AI 执行入口共享
- 前端按相同协议渲染
- 存储层按相同协议提炼摘要

### 11.3 事件持久化策略

首期建议：

1. 实时事件通过 event bus 发给前端
2. 存储层只保存：
   - assistant 最终内容
   - reasoning 摘要
   - search 摘要
   - tool 调用摘要
   - error 摘要

这样既能调试，也避免把大量瞬时事件全量落库。

---

## 12. 多 agent 预留设计

### 12.1 多 agent 边界

本方案不为旧系统保留“以后再说”的兼容表达，但仍明确边界：

1. 新 Runtime 必须直接定义 delegation、background task、subagent profile 的结构位置；
2. 不接入 ACP / 外部 control-plane；
3. 多 agent 能力以本地 Runtime 内协同为目标，而不是分布式控制面。

### 12.2 需要预留的接口

直接定义两类接口：

1. `DelegationSlot`
   - 表示某次执行中可挂接子任务决策点
2. `BackgroundTaskHandle`
   - 表示某个异步背景执行单元的句柄

最小接口形状：

```rust
pub struct DelegationSlot {
    pub slot_name: String,
    pub allowed_profile_ids: Vec<String>,
}

pub struct BackgroundTaskHandle {
    pub task_id: String,
    pub status: String,
}
```

它们属于 Runtime 正式接口，而不是临时占位类型。

### 12.3 未来扩展路径

在统一运行时内继续扩展：

1. `subagent.explore`
2. `subagent.summarize`
3. `subagent.verify`
4. 背景任务树
5. 必要时评估 ACP/bridge

---

## 13. 重构落地计划

### 13.1 一次性重构目标

本次方案不采用旧入口接入中台的迁移模式，而是直接完成以下替换：

1. 以 `ExecutionOrchestrator` 替换现有分散 AI 执行入口；
2. 以 `PromptAssembler` 替换旧系统 prompt + 模板拼装链；
3. 以 `SessionEvent` 替换 `ChatStreamEvent` 等旧事件路径；
4. 以 `execution_session / execution_message / execution_record` 替换旧会话执行表达；
5. 以内聚后的 `AgenticSearchStage` 替换旧 `AgenticSearchOrchestrator` 外围编排。

### 13.2 重构完成标准

1. `chat / grading / communication / search` 四类入口全部统一进入 `ExecutionOrchestrator`；
2. 旧 `chat_with_ai`、旧 `chat_stream`、旧 search 外围编排路径全部删除；
3. 新前端消息渲染只消费 `SessionEvent v1`；
4. Prompt 只允许通过 `PromptAssembler` 生成；
5. Tool visibility 只允许通过 `ToolExposureService` 决定；
6. 事件、结果、失败摘要全部落到新执行存储模型。

### 13.3 验收标准

1. 不存在旧命令适配器、旧 prompt 拼装链、旧 search 编排器残留；
2. Rust 集成测试覆盖流式、非流式、search、tool call、error 五类主链路；
3. 前端卡片渲染仅依赖 `SessionEvent v1` 与标准化渲染状态；
4. `ModelRoutingService` 可根据 capability 明确选模并记录摘要；
5. `ToolExposureService` 可输出会话级工具视图，并对 skill / MCP 一视同仁；
6. 新执行存储模型可回放关键执行轨迹；
7. 对 `text_only` 与 `multimodal` 请求存在可验证的分流与失败保护：multimodal 绝不回落到 text-only。

---

## 14. 错误处理设计

### 14.1 原则

1. 错误既要返回给前端，也要留摘要给存储层
2. 流式中断不能导致“执行消息已创建但无失败痕迹”
3. 所有用户可见错误信息保持中文自然语言

### 14.2 处理路径

建议：

1. 组件内部统一返回 `Result<T, OrchestrationError>`
2. `ExecutionOrchestrator` 捕获后：
   - 发 `Error` 事件
   - 记录失败摘要
   - 更新 execution message 状态/内容
3. Runtime 对外只暴露统一错误边界，由顶层命令层映射到 IPC 返回

### 14.3 典型错误分类

- Provider 连接失败
- 模型能力不匹配
- ToolExposure 为空或工具不可用
- MCP 连接失败
- Prompt 组装失败
- 存储写入失败

建议统一错误类型：

```rust
pub enum OrchestrationError {
    InvalidRequest(String),
    ProfileNotFound(String),
    PromptAssembleFailed(String),
    ModelRoutingFailed(String),
    ToolExposureFailed(String),
    ProviderExecutionFailed(String),
    McpExecutionFailed(String),
    StoreFailed(String),
}
```

映射规则：

1. 内部错误统一保留技术细节；
2. 用户可见错误统一转中文自然语言；
3. `Error` 事件只输出用户可见文本；
4. `execution_record.error_message` 保存可审计摘要，不存敏感凭据。

建议映射辅助函数：

```rust
fn map_orchestration_error(error: OrchestrationError) -> AppError
```

---

## 15. 测试策略

### 15.1 组件单测

覆盖：

- AgentProfileRegistry
- PromptAssembler
- ToolExposureService
- ModelRoutingService

验收门槛：

1. 每个核心组件至少覆盖正常路径 + 1 条异常路径；
2. PromptAssembler 必须有完整组装快照测试；
3. ToolExposureService 必须有 profile/risk/capability 三类过滤测试。

### 15.2 编排层集成测试

覆盖：

- `ExecutionRequest -> ExecutionPlan`
- 流式 / 非流式共用链路
- search / tool / prompt / model 路由协同

验收门槛：

1. 至少覆盖 chat streaming、chat non-streaming、search 三条主链路；
2. 断言事件顺序、最终 message 内容、execution_record 摘要一致。

### 15.3 会话事件回放测试

覆盖：

- `Start -> ThinkingStatus -> Chunk -> Complete`
- 工具调用场景
- 搜索场景
- 失败场景

这是本方案最关键的一类测试，因为它直接验证前端卡片化 UI 与后端统一事件协议是否稳定。

回放验收门槛：

1. 可从持久化摘要还原关键执行轨迹；
2. 前端在 `v1` 事件协议下可稳定渲染所有已定义事件；
3. 错误回放场景必须存在。

---

## 16. 风险与约束

### 16.1 主要风险

1. **抽象过度**：如果第一版把 runtime 做得太重，反而会拖慢交付。
2. **重构切换窗口风险**：单轨替换要求前后端与存储模型在同一轮完成切换。
3. **事件协议频繁变化**：会让前端卡片渲染和后端存储都不稳定。
4. **Provider Runtime 过早膨胀**：容易偏离首期目标。

### 16.2 控制策略

1. 第一版先交付完整单轨 Runtime 主执行链，本地 subagent 接口直接落在新 Runtime 内。
2. 先冻结 SessionEvent 协议，再通过 adapter 层集中演进字段。
3. 先做 capability router，不做 control-plane。
4. 完成标准是“新 Runtime 独立闭环且旧执行链已删除”，而不是抽象存在但旧链仍在运行。

### 16.3 性能边界（首期目标）

1. 非流式执行不额外增加一次数据库往返以上的固定开销；
2. 流式首包延迟相较现有实现增加不超过 150ms；
3. `execution_record` 摘要字段默认控制在 16KB 内，避免把执行日志当正文存储。

---

## 17. 与当前仓库的映射关系

### 17.1 需要下沉为新 Runtime 依赖的现有模块

- `services/llm_provider.rs`（保留 provider client 能力，但重写其上层调用方式）
- `services/skill.rs`（保留 skill 元数据与执行能力，但重写加载与暴露方式）
- `services/skill_tool_adapter.rs`（保留可桥接能力，但由 `ToolExposureService` 重新统筹）
- `services/tool_registry.rs`（保留为底层全局工具源）
- `services/mcp_runtime.rs`（保留底层 MCP client 能力）
- `services/mcp_tool_adapter.rs`（保留底层桥接能力）

### 17.2 需要删除或被新 Runtime 取代的现有模块

- `commands/chat.rs`（由统一 execution command 重写替换）
- `services/prompt_template_registry.rs` 的旧运行链（其能力并入 `PromptAssembler`）
- `services/agentic_search.rs` / `services/agentic_search_agent.rs` 的旧外围编排路径
- `models/conversation.rs` 中仅面向旧 chat stream 的执行表达

### 17.3 需要新增的模块（建议方向）

- `services/ai_orchestration/` 目录
- `execution_orchestrator.rs`
- `agent_profile_registry.rs`
- `prompt_assembler.rs`
- `tool_exposure.rs`
- `model_routing.rs`
- `session_event_bus.rs`
- `execution_store.rs`
- `execution_types.rs`
- `execution_command.rs`

### 17.4 需要直接重构的入口

- `chat`
- `grading`
- `communication`
- `search`

---

## 18. 待确认问题

本轮设计中已确认的大方向如下：

1. 采用中台式编排层（方案 B）
2. 四类 AI 入口一次性统一到新 Runtime 主链
3. Provider Runtime 直接按 router/catalog/capability metadata 完整重构，但不引入 ACP / control-plane
4. 多 agent 直接定义本地 Runtime 接口边界，不保留旧执行链

在进入实现计划前，仍建议明确两项实施偏好（非架构阻塞项）：

1. `execution_record.metadata_json` 中的轻量快照字段是否还需要进一步收紧
2. 前端 `SessionEvent v1` 归一化层是否命名为 `ExecutionEventNormalizer`

### 18.1 推荐结论：`metadata_json` 首期允许存“轻量快照”，不存可重建主数据

建议：

1. **允许**在 `execution_record.metadata_json` 中存轻量快照；
2. **禁止**把可从结构化字段重建的主数据重复塞入 `metadata_json`；
3. 首期只允许以下内容进入快照：
   - `profile_id`
   - `output_protocol`
   - `stream_mode`
   - `search_enabled`
   - `selected_tool_names`
   - `fallback_used`

原因：

1. 这些信息对调试和审计有价值；
2. 它们属于“执行时上下文”，不适合再单独拆表；
3. 若把完整 profile 解析结果、完整 prompt、完整事件流都放进去，会迅速让 `metadata_json` 失控。

明确禁止放入的内容：

1. 完整 assembled prompt 正文
2. 全量搜索原文
3. 全量 tool input/output 明细
4. 可从 `execution_record` 结构化字段直接重建的重复信息

推荐结构示例：

```json
{
  "profile_id": "chat.homeroom",
  "output_protocol": "markdown",
  "stream_mode": "streaming",
  "search_enabled": true,
  "selected_tool_names": ["search.student", "search.memory"],
  "fallback_used": false
}
```

### 18.2 推荐结论：前端采用独立 SessionEvent adapter 作为新协议归一化层

建议：

1. 前端新增一层 `ExecutionEventNormalizer`；
2. 所有后端事件先经 adapter 归一化，再进入新的消息卡片渲染状态；
3. adapter 不是兼容旧协议，而是新协议到 UI 状态的唯一归一化入口。

不推荐把归一化逻辑散落在各组件里的原因：

1. 事件种类已经不少，协议演进时会把条件分支散到多个组件；
2. 现在已经有思考、工具调用、搜索摘要、正文卡片等多种 UI 形态，统一入口更稳；
3. adapter 更适合做协议归一化、字段收敛和默认值处理。

推荐前端流转：

```text
Raw SessionEvent(v1)
  -> ExecutionEventNormalizer
  -> NormalizedChatEvent
  -> ChatMessage / card renderer
```

推荐规范：

1. 后端永远发送带 `version` 的原始事件；
2. adapter 负责版本分发与默认值补齐；
3. UI 组件不直接判断 `version`；
4. 若未来出现 `v2`，只改 adapter 层，不改卡片组件主逻辑。

### 18.3 实施计划默认采用的口径

若用户未再单独指定，实现计划默认按以下口径展开：

1. `execution_record.metadata_json` 采用“轻量快照”策略；
2. 前端事件归一化采用独立 `ExecutionEventNormalizer`；
3. 两者都视为 Runtime 首版必须纳入的基础约束，而不是后补优化项。

---

## 19. 推荐的下一步

建议后续直接进入实现计划拆解，优先级如下：

1. `WP-AI-012 Agent Runtime / Orchestration Layer`
2. `WP-AI-013 Prompt Assembler + AgentProfileRegistry`
3. `WP-AI-014 SessionEventBus + ExecutionStore`
4. `WP-AI-015 ToolExposureService + 会话级 MCP/Skill 暴露`
5. `WP-AI-016 ModelRoutingService 与 Provider Catalog`

其中第一批实施应以“新 Runtime 独立闭环 + 旧执行链全部删除”为验收样板，而不是旧入口接入新中台。
          
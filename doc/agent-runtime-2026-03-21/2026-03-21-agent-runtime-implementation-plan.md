# PureWorker Agent Runtime / Orchestration Layer Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a single-track AI runtime for PureWorker that replaces the current scattered execution paths for chat, search, prompt assembly, event emission, and execution persistence.

**Architecture:** Replace the existing chat/search-centric execution flow with a unified `ExecutionOrchestrator` pipeline. Existing provider, skill, MCP, and tool registry capabilities stay as low-level dependencies, while prompt assembly, tool exposure, model routing, session events, and execution storage move into a new orchestration module tree.

**Tech Stack:** Rust, Tauri 2, Rig, SQLx + SQLite, Specta, React + TypeScript, Tailwind CSS

---

## Preconditions

- Confirm design spec exists: `doc/agent-runtime-2026-03-21/2026-03-21-agent-runtime-design.md`
- Confirm plan target exists: `doc/agent-runtime-2026-03-21/2026-03-21-agent-runtime-implementation-plan.md`
- Confirm latest migration number is `0015_audit_detail.sql`, so the new migration should be `0016_execution_runtime.sql`
- Confirm service tests in this repo follow inline Rust `#[cfg(test)] mod tests` style rather than a nested `tests/` directory under services
- Confirm prompt template sources exist in `packages/prompt-templates/templates/*.toml`

If any precondition fails, stop and correct the plan before implementation.

---

## File Structure

### Existing files to modify

- `apps/desktop/src-tauri/src/commands/chat.rs` — remove old chat execution flow and switch to new execution command path
- `apps/desktop/src-tauri/src/commands/mod.rs` — register new execution command module
- `apps/desktop/src-tauri/src/lib.rs` — wire the new runtime into Tauri setup and command registration
- `apps/desktop/src-tauri/src/models/conversation.rs` — remove old chat-stream-specific execution protocol and introduce or slim down shared conversation-only pieces
- `apps/desktop/src-tauri/src/models/mod.rs` — export new execution models
- `apps/desktop/src-tauri/src/services/mod.rs` — export new orchestration modules
- `apps/desktop/src-tauri/src/services/llm_provider.rs` — keep provider client capability but route orchestration callers through model/runtime selection interfaces
- `apps/desktop/src-tauri/src/services/prompt_template_registry.rs` — stop serving as runtime prompt chain and expose only reusable task/template data
- `apps/desktop/src-tauri/src/services/skill_tool_adapter.rs` — make it a low-level tool bridge consumed by ToolExposureService
- `apps/desktop/src-tauri/src/services/tool_registry.rs` — remain the global tool source used by ToolExposureService
- `apps/desktop/src-tauri/src/services/mcp_runtime.rs` — remain low-level MCP client/runtime
- `apps/desktop/src-tauri/src/services/mcp_tool_adapter.rs` — remain MCP tool bridge consumed by ToolExposureService
- `apps/desktop/src-tauri/src/services/conversation_service.rs` — either split or reduce to conversation-only CRUD after execution storage moves out
- `apps/desktop/src-tauri/src/services/agentic_search.rs` — fold search orchestration logic into execution stage logic
- `apps/desktop/src-tauri/src/services/agentic_search_agent.rs` — remove old external orchestrator wrapper behavior
- `apps/desktop/src/components/chat/ChatPanel.tsx` — switch frontend streaming consumption to new execution event contract
- `apps/desktop/src/components/chat/ChatMessage.tsx` — map normalized execution events to cards
- `apps/desktop/src/components/chat/ChatMessageList.tsx` — render only normalized runtime events/messages

### Existing files to review during implementation

- `apps/desktop/src-tauri/src/models/ai_config.rs`
- `apps/desktop/src-tauri/src/commands/ai_generation.rs`
- `apps/desktop/src-tauri/src/commands/assignment_grading.rs`
- `apps/desktop/src-tauri/src/commands/parent_communication.rs`
- `apps/desktop/src-tauri/src/services/ai_generation.rs`
- `apps/desktop/src-tauri/src/services/multimodal_grading.rs`
- `apps/desktop/src-tauri/src/services/parent_communication.rs`
- `apps/desktop/src/bindings.ts`

### New backend files to create

- `apps/desktop/src-tauri/src/commands/execution.rs` — new unified Tauri execution entrypoint
- `apps/desktop/src-tauri/src/models/execution.rs` — execution session/message/record/event/request/result types
- `apps/desktop/src-tauri/src/services/ai_orchestration/mod.rs` — orchestration module exports
- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs` — main runtime pipeline
- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_request_factory.rs` — build execution requests from command inputs
- `apps/desktop/src-tauri/src/services/ai_orchestration/agent_profile_registry.rs` — persistent runtime profile loading
- `apps/desktop/src-tauri/src/services/ai_orchestration/prompt_assembler.rs` — runtime prompt assembly
- `apps/desktop/src-tauri/src/services/ai_orchestration/tool_exposure.rs` — session-level tool view logic
- `apps/desktop/src-tauri/src/services/ai_orchestration/model_routing.rs` — capability-based model routing
- `apps/desktop/src-tauri/src/services/ai_orchestration/provider_catalog.rs` — provider registry / model catalog / discovery-fallback 合并
- `apps/desktop/src-tauri/src/services/ai_orchestration/session_event_bus.rs` — event emission and persistence boundary
- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_store.rs` — execution session/message/record persistence
- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_stage.rs` — stage trait + search stage contract
- `apps/desktop/src-tauri/src/services/ai_orchestration/error.rs` — orchestration error types and AppError mapping

### New frontend files to create

- `apps/desktop/src/components/chat/execution-event-normalizer.ts` — normalize raw runtime events into UI event state
- `apps/desktop/src/components/chat/types.ts` — shared normalized chat/runtime UI types if existing types are too coupled

### New database / test files to create

- `apps/desktop/src-tauri/migrations/0016_execution_runtime.sql` — execution session/message/record schema
- Inline `#[cfg(test)] mod tests` blocks inside each orchestration Rust service file, matching current repo convention

---

## Chunk 1: Runtime data model and storage foundation

### Task 1: Add execution runtime database schema

**Files:**
- Create: `apps/desktop/src-tauri/migrations/0016_execution_runtime.sql`
- Review: `apps/desktop/src-tauri/src/models/conversation.rs`
- Review: `apps/desktop/src-tauri/src/services/conversation_service.rs`

- [ ] **Step 1: Write the failing storage test plan in comments or test scaffold**

Define expected persisted entities for:
- one execution session
- one execution message for assistant output
- one execution record with summaries and status

- [ ] **Step 2: Add migration for new execution tables**

Include tables for:
- `execution_session`
- `execution_message`
- `execution_record`

Ensure all tables follow repo rules:
- TEXT UUID primary keys
- `is_deleted INTEGER NOT NULL DEFAULT 0` when applicable
- ISO 8601 timestamps
- business queries constrained to non-deleted rows

- [ ] **Step 3: Run migration validation path**

Run: `cargo test`
Expected: existing DB-related tests still pass; new schema compiles into SQLx usage later

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/migrations/0016_execution_runtime.sql
git commit -m "feat: 新增执行运行时数据表"
```

### Task 2: Create execution model types

**Files:**
- Create: `apps/desktop/src-tauri/src/models/execution.rs`
- Modify: `apps/desktop/src-tauri/src/models/mod.rs`
- Review: `apps/desktop/src-tauri/src/models/ai_config.rs`
- Review: `apps/desktop/src-tauri/src/models/conversation.rs`

- [ ] **Step 1: Write failing unit tests or compile-time references for new types**

Cover these types:
- `ExecutionRequest`
- `ExecutionResponse`
- `ExecutionSession`
- `ExecutionMessage`
- `ExecutionRecord`
- `SessionEvent`
- `ExecutionStatus`
- `ExecutionEntrypoint`

- [ ] **Step 2: Implement the new execution model file**

Define exact fields from the design doc, including:
- versioned `SessionEvent`
- lightweight metadata snapshot support
- Chinese comments at file and method/type level per repo rules

- [ ] **Step 3: Export the models in `models/mod.rs`**

Ensure downstream services can import them without path hacks.

- [ ] **Step 4: Run type/build validation**

Run: `cargo test`
Expected: compile succeeds with new model exports

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/models/execution.rs apps/desktop/src-tauri/src/models/mod.rs
git commit -m "feat: 新增执行运行时模型定义"
```

---

## Chunk 2: Orchestration module skeleton

### Task 3: Create orchestration module tree and error boundary

**Files:**
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/mod.rs`
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/error.rs`
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/execution_stage.rs`
- Modify: `apps/desktop/src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write failing compile references for module exports**

Reference the new modules from `services/mod.rs` before implementation.

- [ ] **Step 2: Implement orchestration module exports and shared traits**

Define:
- orchestration error enum
- AppError mapping helper
- execution stage trait
- shared trait exports for registry/assembler/router/store/event bus

- [ ] **Step 3: Validate compile**

Run: `cargo test`
Expected: new orchestration module tree compiles cleanly

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/services/mod.rs apps/desktop/src-tauri/src/services/ai_orchestration/
git commit -m "feat: 搭建执行运行时服务模块骨架"
```

### Task 4: Build AgentProfileRegistry with persistent runtime profiles

**Files:**
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/agent_profile_registry.rs`
- Review: `doc/agent-runtime-2026-03-21/2026-03-21-agent-runtime-design.md`

- [ ] **Step 1: Write failing tests for profile loading and lookup**

Test cases:
- can load required built-in profiles
- rejects unknown profile id
- profile tool risk and output protocol are available at runtime

- [ ] **Step 2: Implement persistent-ready profile registry**

Start with a runtime-owned profile source backed by the new runtime persistence model. Do not hardcode behavior into old role branches. At minimum, the registry must expose the profiles described in the spec for:
- `chat.homeroom`
- `chat.grading`
- `chat.communication`
- `chat.ops`
- `search.agentic`

- [ ] **Step 3: Run focused tests**

Run: `cargo test agent_profile_registry`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/services/ai_orchestration/agent_profile_registry.rs
git commit -m "feat: 新增运行时 AgentProfile 注册表"
```

Because this repo uses inline Rust tests, put tests in `agent_profile_registry.rs` under `#[cfg(test)] mod tests` and stage only that file unless a shared test helper file is truly needed.

### Task 5: Implement ModelRoutingService

**Files:**
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/model_routing.rs`
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/provider_catalog.rs`
- Review: `apps/desktop/src-tauri/src/services/llm_provider.rs`
- Review: `apps/desktop/src-tauri/src/models/ai_config.rs`

- [ ] **Step 1: Write failing tests for capability-based selection**

Test cases:
- profile-specified model wins
- entrypoint/capability maps to correct default model field
- missing dedicated model config falls back to `default_model` within the same capability tier
- insufficient capability returns error
- multimodal request never falls back to text-only model
- text-only request defaults to text model path and only enters multimodal fallback when explicit config allows

- [ ] **Step 2: Implement routing service and selected model type**

Use current provider config as input, but expose an opencode-aligned runtime API:

1. `SelectedModel` carries `provider_id + model_id + capability snapshot + fallback_used`.
2. routing output includes machine-readable `RoutingTrace` for auditing.
3. capability check is hard-gate; return explicit error (e.g. `ModelCapabilityInsufficient`) instead of silent downgrade.

- [ ] **Step 3: Implement ProviderCatalog / LoaderMatrix baseline**

Build a first implementation that aligns with opencode provider runtime mechanism:

1. provider registry with enabled/health status
2. model catalog merge path: discovery result + static fallback
3. provider loader matrix abstraction (openai-compatible / anthropic-native / custom-compatible)
4. catalog metadata fields at least include `input_modalities`, `supports_text_input`, `supports_image_input`, `supports_tool_calling`, `supports_reasoning`, `context_window`, `max_output_tokens`

> **审核修正 (P0)**：
> 1. 当前 `provider_adapter/mod.rs` 的 `from_provider_name("gemini")` 返回 `None`，需在 ProviderCatalog 构建前修复（已在审核中修正代码，此处确认计划覆盖）。
> 2. 当前 `llm_provider.rs` 的 `get_vision_model()` 在 vision model 未配置时静默回落到 text model，违反设计硬约束「multimodal 绝不回落到 text-only」。新 ModelRoutingService 必须返回 `ModelCapabilityInsufficient` 错误而非静默回落。Step 1 的测试用例已覆盖此场景。

- [ ] **Step 4: Run focused tests**

Run: `cargo test model_routing`
Expected: PASS

Run: `cargo test provider_catalog`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/services/ai_orchestration/model_routing.rs apps/desktop/src-tauri/src/services/ai_orchestration/provider_catalog.rs
git commit -m "feat: 新增执行运行时模型路由服务"
```

Put tests inline in `model_routing.rs` and `provider_catalog.rs` under `#[cfg(test)] mod tests`.

---

## Chunk 3: Prompt and tool runtime

### Task 6: Rebuild prompt assembly around runtime layers

**Files:**
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/prompt_assembler.rs`
- Modify: `apps/desktop/src-tauri/src/services/prompt_template_registry.rs`
- Review: `packages/prompt-templates/templates/*.toml`

- [ ] **Step 1: Write failing prompt snapshot tests**

Cover:
- homeroom chat assembly
- grading assembly
- search-enabled assembly
- tool-summary inclusion

- [ ] **Step 1.5: 盘点现有模板资源**

扫描 `packages/prompt-templates/templates/*.toml`，列出所有活跃业务模板，确认每个模板在新 PromptAssembler 中的 task layer 入口映射。若某个模板无法映射，在此步骤中标记为需设计扩展。

- [ ] **Step 2: Refactor `prompt_template_registry.rs` into template data provider only**

Remove its role as the runtime assembly pipeline.

Template source mapping for the first implementation:
- `chat.homeroom` -> `chat_homeroom_text.toml` / `chat_homeroom_multimodal.toml`
- `chat.grading` -> `grading_multimodal_json.toml`
- `chat.communication` -> `parent_communication.toml`
- `chat.ops` -> define runtime system/profile layer even if no dedicated template file exists yet
- announcements/comments flows -> `activity_announcement.toml`, `semester_comment.toml`

Variant selection rule (mandatory):
- choose `*_multimodal` template only when selected model capability explicitly declares multimodal input support
- for text-only requests, force text template and reject implicit multimodal promotion

- [ ] **Step 3: Implement PromptAssembler**

Compose layers in the exact spec order:
- system
- profile
- task
- evidence
- tool summary
- output protocol
- user input

- [ ] **Step 4: Run focused tests**

Run: `cargo test prompt_assembler`
Expected: PASS with stable snapshots

Must include assertions for:
- text-only request -> text template
- multimodal request -> multimodal template
- multimodal request + no capable model -> explicit failure path

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/services/prompt_template_registry.rs apps/desktop/src-tauri/src/services/ai_orchestration/prompt_assembler.rs
git commit -m "feat: 重构提示词装配为运行时组装链"
```

Put tests inline in `prompt_assembler.rs` under `#[cfg(test)] mod tests`.

### Task 7: Build ToolExposureService

**Files:**
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/tool_exposure.rs`
- Review: `apps/desktop/src-tauri/src/services/tool_registry.rs`
- Review: `apps/desktop/src-tauri/src/services/skill_tool_adapter.rs`
- Review: `apps/desktop/src-tauri/src/services/mcp_tool_adapter.rs`

- [ ] **Step 1: Write failing tests for tool filtering**

Cover:
- allowlist/denylist
- risk ceiling
- entrypoint capability filtering
- MCP enabled/health filtering

- [ ] **Step 2: Implement session-level tool view generation**

Build one `SessionToolView` path for builtin / skill / MCP.

- [ ] **Step 3: Run focused tests**

Run: `cargo test tool_exposure`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/services/ai_orchestration/tool_exposure.rs
git commit -m "feat: 新增会话级工具暴露服务"
```

Put tests inline in `tool_exposure.rs` under `#[cfg(test)] mod tests`.

---

## Chunk 4: Event bus, execution store, and search stage

### Task 8: Implement execution storage service

**Files:**
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/execution_store.rs`
- Modify: `apps/desktop/src-tauri/src/services/conversation_service.rs`
- Review: `apps/desktop/src-tauri/src/models/execution.rs`

- [ ] **Step 1: Write failing storage tests**

Cover:
- create execution session
- create execution message
- finalize success
- record failure summary

- [ ] **Step 2: Implement execution store and reduce old conversation service responsibility**

Move execution persistence out of `ConversationService`.

- [ ] **Step 3: Run focused tests**

Run: `cargo test execution_store`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/services/ai_orchestration/execution_store.rs apps/desktop/src-tauri/src/services/conversation_service.rs
git commit -m "feat: 新增执行运行时存储服务"
```

### Task 9: Implement SessionEventBus and search stage

**Files:**
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/session_event_bus.rs`
- Modify: `apps/desktop/src-tauri/src/services/agentic_search.rs`
- Modify: `apps/desktop/src-tauri/src/services/agentic_search_agent.rs`
- Review: `apps/desktop/src-tauri/src/commands/chat.rs`

> **审核补充 (P2)**：当前 `execution.rs` 中的 `ExecutionResponse` 是前端 IPC 返回用的精简结构（仅含 content/model/status）。本 Task 实现 `ExecutionOrchestrator` 时，内部执行链应使用完整的 `ExecutionResult`（含 tool_calls_summary、search_summary、reasoning_summary、error_message），仅在 IPC 命令层将 `ExecutionResult` 映射为精简 `ExecutionResponse`。

- [ ] **Step 1: Write failing event/replay tests**

Cover:
- event order for streaming response
- search summary + reasoning events
- error event persistence path

- [ ] **Step 2: Implement SessionEventBus**

Do not reuse old `chat-stream`-specific logic as the runtime contract.

- [ ] **Step 3: Convert agentic search into runtime stage logic**

Delete or neutralize the old external orchestrator flow.

Expected search stage behavior:
- trigger when `ExecutionRequest.use_agentic_search == true` or the selected profile requires search
- read evidence before prompt assembly
- produce `SearchSummary` + `Reasoning` events into the runtime event bus
- never emit UI-specific events outside the runtime bus

- [ ] **Step 4: Run focused tests**

Run: `cargo test session_event_replay`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/services/ai_orchestration/session_event_bus.rs apps/desktop/src-tauri/src/services/agentic_search.rs apps/desktop/src-tauri/src/services/agentic_search_agent.rs
git commit -m "feat: 统一执行事件总线并内聚检索阶段"
```

---

## Chunk 5: Main orchestrator and command replacement

### Task 10: Implement ExecutionOrchestrator

> **审核修正 (P1)**：当前 `chat.rs:456-475` 的流式实现是假流式（生成完整响应后按句分割 + 30ms 延迟逐句发）。本 Task 替换为 provider adapter 真实流式推送，而非保留此模拟行为。

**Files:**
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs`
- Create: `apps/desktop/src-tauri/src/services/ai_orchestration/execution_request_factory.rs`
- Review: all orchestration module files from previous tasks

- [ ] **Step 0: Rig 流式能力 spike（原型验证）**

在开始实现前，用最小代码验证 `rig-core` 当前版本是否支持：
1. 流式 chat completion（`CompletionModel::stream()` 或等价路径）
2. 流式 + tool calling 协同工作
3. 流式中断时的错误处理边界

若 Rig 不支持所需流式能力，在此步骤中记录限制并提出替代方案（如直接调用 provider HTTP streaming API）后再继续。

- [ ] **Step 1: Write failing integration tests for orchestrator**

Cover:
- non-streaming chat
- streaming chat
- search-enabled request
- tool-enabled request
- failure path with persisted summary

- [ ] **Step 2: Implement execution plan generation and orchestrator pipeline**

Pipeline must:
- validate request
- load profile
- select model
- compute session tool view
- optionally run search stage
- assemble prompt
- execute provider call
- emit events
- persist execution result

- [ ] **Step 3: Run focused integration tests**

Run: `cargo test execution_orchestrator`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs apps/desktop/src-tauri/src/services/ai_orchestration/execution_request_factory.rs
git commit -m "feat: 新增统一执行编排主链"
```

Put tests inline in `execution_orchestrator.rs` under `#[cfg(test)] mod tests`.

### Task 11: Replace chat command with execution command

**Files:**
- Create: `apps/desktop/src-tauri/src/commands/execution.rs`
- Modify: `apps/desktop/src-tauri/src/commands/chat.rs`
- Modify: `apps/desktop/src-tauri/src/commands/mod.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing command-level tests or compile assertions**

Ensure command registration references new execution entrypoints.

- [ ] **Step 2: Introduce new execution command and remove old chat-owned orchestration logic**

Expected outcome:
- chat command becomes thin or is removed from primary execution path
- execution command owns runtime invocation

- [ ] **Step 3: Re-export bindings if command surface changes**

Run: `pnpm bindings:export`
Expected: `apps/desktop/src/bindings.ts` updated if IPC changed

- [ ] **Step 4: Run Rust validation**

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/commands apps/desktop/src-tauri/src/lib.rs apps/desktop/src/bindings.ts
git commit -m "feat: 以统一执行命令替换旧聊天执行链"
```

---

## Chunk 6: Frontend runtime event switch

### Task 12: Normalize runtime events for UI rendering

**Files:**
- Create: `apps/desktop/src/components/chat/execution-event-normalizer.ts`
- Create or Modify: `apps/desktop/src/components/chat/types.ts`
- Modify: `apps/desktop/src/components/chat/ChatPanel.tsx`
- Modify: `apps/desktop/src/components/chat/ChatMessageList.tsx`
- Modify: `apps/desktop/src/components/chat/ChatMessage.tsx`

- [ ] **Step 1: Write failing UI/state tests or normalization fixtures**

Cover:
- thinking card event
- tool call/result card event
- reasoning summary event
- chunk-to-message accumulation
- error rendering

If no frontend test harness exists yet, create fixture-driven normalization assertions first and defer full component tests until a minimal test runner is available.

- [ ] **Step 2: Implement `ExecutionEventNormalizer`**

Convert raw `SessionEvent v1` into UI-facing normalized state only.

- [ ] **Step 3: Update chat components to consume normalized runtime events**

Remove dependence on legacy event semantics.

- [ ] **Step 4: Run frontend validation**

Run: `pnpm build`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/components/chat
git commit -m "feat: 前端切换到统一执行事件渲染"
```

---

## Chunk 7: Broaden the runtime to other AI entrypoints

### Task 13: Route grading and communication through runtime

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/assignment_grading.rs`
- Modify: `apps/desktop/src-tauri/src/commands/ai_generation.rs`
- Modify: `apps/desktop/src-tauri/src/commands/parent_communication.rs`
- Modify: `apps/desktop/src-tauri/src/services/multimodal_grading.rs`
- Modify: `apps/desktop/src-tauri/src/services/ai_generation.rs`
- Modify: `apps/desktop/src-tauri/src/services/parent_communication.rs`

- [ ] **Step 1: Write failing integration tests for non-chat runtime entrypoints**

Cover:
- grading request resolves grading profile + prompt path
- communication request resolves communication profile + output protocol
- grading attachments trigger multimodal routing and never degrade to text-only model
- communication/text requests stay on text-only routing path unless profile explicitly requests multimodal

- [ ] **Step 2: Repoint command/service execution to `ExecutionOrchestrator`**

No direct prompt assembly or ad-hoc execution should remain.

- [ ] **Step 3: Run Rust validation**

Run: `cargo test`
Expected: all affected AI entrypoints pass

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/commands/assignment_grading.rs apps/desktop/src-tauri/src/commands/ai_generation.rs apps/desktop/src-tauri/src/commands/parent_communication.rs apps/desktop/src-tauri/src/services/multimodal_grading.rs apps/desktop/src-tauri/src/services/ai_generation.rs apps/desktop/src-tauri/src/services/parent_communication.rs
git commit -m "feat: 统一批改与沟通类 AI 入口到运行时主链"
```

---

## Chunk 8: Final verification and cleanup

### Task 14: Remove dead legacy paths and verify end-to-end behavior

> **审核补充（风险控制）**：在开始删除旧代码之前，新 Runtime 执行链必须已通过全部集成测试。建议在此处打 git tag（如 `pre-cleanup-checkpoint`）作为回滚点，确保单轨替换出问题时可快速恢复。

- [ ] **Step 0: 创建回滚检查点**

确认新 Runtime 全部集成测试通过后，执行：
```bash
git tag pre-cleanup-checkpoint
```

**Files:**
- Modify/Delete: legacy logic identified in `commands/chat.rs`, `services/agentic_search*.rs`, `services/prompt_template_registry.rs`, `models/conversation.rs`
- Review: `apps/desktop/src/bindings.ts`
- Review: `doc/agent-runtime-2026-03-21/2026-03-21-agent-runtime-design.md`

- [ ] **Step 1: Remove remaining dead code and legacy-only comments**

Delete anything that exists only to support the old scattered execution flow.

- [ ] **Step 2: Run diagnostics on all touched Rust/TS files**

Use language diagnostics until zero errors remain.

- [ ] **Step 3: Run full backend validation**

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
Expected: PASS

- [ ] **Step 4: Run full frontend validation**

Run: `pnpm build`
Expected: PASS

- [ ] **Step 5: Update bindings if needed and re-check git diff**

Run: `pnpm bindings:export`
Expected: bindings synced with final command surface

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src-tauri apps/desktop/src apps/desktop/src/bindings.ts
git commit -m "feat: 完成统一 Agent Runtime 重构"
```

---

## Global Validation Checklist

- [ ] `cargo fmt --check`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo test`
- [ ] `pnpm build`
- [ ] `pnpm bindings:export`
- [ ] Confirm no remaining direct chat-owned orchestration path
- [ ] Confirm no remaining legacy `ChatStreamEvent`-only runtime dependency
- [ ] Confirm no remaining external `AgenticSearchOrchestrator` execution path

---

## Execution Notes

- Prefer Rust-first refactor order: schema/models -> orchestration traits/services -> command replacement -> frontend event switch -> other AI entrypoints -> cleanup.
- Follow repo rule: all user-facing error messages must be Chinese.
- Follow repo rule: add file-level and method-level Chinese comments in production code.
- After command surface changes, always regenerate `apps/desktop/src/bindings.ts`.
- Do not preserve legacy execution paths “temporarily”; this plan assumes single-track replacement.

# PureWorker Agent Runtime 业务修复工作包拆解

> 生成时间：2026-03-22  
> 来源文档：`doc/agent-runtime-2026-03-21/business-runtime-adoption-verification-and-repair-plan-2026-03-22.md`  
> 目标：将业务修复计划拆分为**可直接执行、可排期、可验收**的工作包  
> 使用方式：每个工作包都可以单独立项、单独开发、单独验收；建议严格按依赖顺序推进

---

## 1. 工作包总览

本次拆分遵循两个原则：

1. **先补主链闭环，再补扩展能力**；
2. **先解决“业务能不能真的跑起来”，再解决“能力是否先进”**。

共拆分为 10 个可直接执行的工作包：

| 编号 | 名称 | 优先级 | 目标 | 依赖 |
|---|---|---|---|---|
| WP-AI-BIZ-001 | 聊天业务统一运行时闭环 | P0 | 去掉兼容壳，统一聊天 UI 与事件消费 | 无 |
| WP-AI-BIZ-002 | Agentic Search 业务接入与搜索摘要事件化 | P0 | 让检索真正进入聊天/搜索主链 | 001 |
| WP-AI-BIZ-003 | ExecutionStore 真实业务落库与审计接线 | P0 | 建立统一执行主账本 | 001,002 |
| WP-AI-BIZ-004 | 生成型业务统一请求工厂与执行来源追踪 | P1 | 统一家校沟通/评语/公告生成链 | 003 |
| WP-AI-BIZ-005 | 学期评语批量任务运行时化 | P2 | 让批量评语变成子执行聚合模型 | 003,004 |
| WP-AI-BIZ-006 | 作业批改主链运行时化 | P2 | 让运行时统领批改链，而非只做评分子步骤 | 003 |
| WP-AI-BIZ-007 | 运行时业务事件补齐与前后端真实贯通 | P0/P1 | 补齐前端已监听但后端未真实产出的事件 | 001,002 |
| WP-AI-BIZ-008 | OCR 真链路接入与模拟路径降级 | P2 | 去掉 OCR 模拟回退对生产链路的依赖 | 006 |
| WP-AI-BIZ-009 | 练习卷生成运行时化评估与接入 | P2/P3 | 决定练习卷是否进入统一运行时 | 004,006 |
| WP-AI-BIZ-010 | MCP / Skill 业务默认接入 | P3 | 让工具面真正进入真实业务链 | 003,007 |

---

## 2. 推荐实施批次

### 批次 A：先把运行时主链“跑通”

包含：

1. `WP-AI-BIZ-001`
2. `WP-AI-BIZ-002`
3. `WP-AI-BIZ-007`
4. `WP-AI-BIZ-003`

目标：

1. 聊天 UI 不再依赖兼容壳；
2. 搜索增强不再是空字段；
3. 前端已监听事件能真正收到数据；
4. 执行记录真正落库。

### 批次 B：把生成型业务统一起来

包含：

1. `WP-AI-BIZ-004`
2. `WP-AI-BIZ-005`

目标：

1. 家校沟通 / 学期评语 / 活动公告统一走请求工厂；
2. 生成结果能追溯执行来源；
3. 批量评语不再只是旧任务循环。

### 批次 C：把批改主链做成真正业务闭环

包含：

1. `WP-AI-BIZ-006`
2. `WP-AI-BIZ-008`

目标：

1. 运行时接管批改主链；
2. OCR 不再默认模拟；
3. 资产级执行轨迹可追踪。

### 批次 D：补 AI 邻接能力和扩展面

包含：

1. `WP-AI-BIZ-009`
2. `WP-AI-BIZ-010`

目标：

1. 练习卷业务是否纳入运行时有明确结论并落地；
2. MCP / Skill 真正变成业务默认可用工具面。

---

## 3. 逐个工作包说明

## WP-AI-BIZ-001：聊天业务统一运行时闭环

**目标**

消除聊天业务的新旧双入口状态，让聊天类 UI 全部以统一运行时事件流为准。

**业务问题**

1. `useChatStream` 已走新链；
2. `AiPanel` 仍走 `chat_with_ai` 兼容壳；
3. 用户在不同聊天入口会看到不同体验。

**范围**

- 前端聊天入口统一
- 后端兼容壳降级/标注废弃
- 聊天消息展示模型统一

**涉及文件**

- `apps/desktop/src/components/layout/AiPanel.tsx`
- `apps/desktop/src/hooks/useChatStream.ts`
- `apps/desktop/src-tauri/src/commands/chat.rs`
- `apps/desktop/src-tauri/src/commands/execution.rs`

**交付物**

1. 聊天 UI 接口统一后的实现代码
2. 兼容壳处理说明
3. 聊天业务回归用例

**验收标准**

1. 所有聊天 UI 都不再展示兼容占位文案；
2. 统一消费 `execution-stream` / `chat-stream` 事件；
3. 同一问题在不同聊天入口得到一致交互流程。

**风险**

1. 前端聊天状态机分裂；
2. 兼容壳删除过早导致旧界面不可用。

---

## WP-AI-BIZ-002：Agentic Search 业务接入与搜索摘要事件化

**目标**

让 `use_agentic_search` 变成真正生效的业务能力，而不是请求字段占位。

**业务问题**

1. 搜索 evidence 仍未进入 prompt；
2. `search_evidence` 仍走旧服务；
3. 搜索摘要没有成为真实用户可见事件。

**范围**

- 统一搜索阶段接入
- evidence 注入 prompt
- SearchSummary/Reasoning 事件真实产出

**涉及文件**

- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs`
- `apps/desktop/src-tauri/src/services/agentic_search_agent.rs`
- `apps/desktop/src-tauri/src/commands/memory_search.rs`
- `apps/desktop/src-tauri/src/services/ai_orchestration/prompt_assembler.rs`

**交付物**

1. 搜索阶段统一接线实现
2. 搜索摘要事件模型
3. 搜索增强聊天回归用例

**验收标准**

1. 开启搜索增强后，prompt 的 `evidence_text` 非占位值；
2. 前端能看到真实 `SearchSummary`；
3. 搜索业务与聊天增强业务共享同一搜索阶段实现。

---

## WP-AI-BIZ-003：ExecutionStore 真实业务落库与审计接线

**目标**

让 `execution_session / execution_message / execution_record` 从“底座表”变成真实业务主账本。

**业务问题**

1. 运行时已接入多个业务；
2. 但执行记录并未真实入库；
3. 结果无法追溯到具体执行。

**范围**

- 聊天执行记录
- 生成执行记录
- 批改执行记录
- conversation 与 execution 的映射策略

**涉及文件**

- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_store.rs`
- `apps/desktop/src-tauri/src/commands/execution.rs`
- `apps/desktop/src-tauri/src/services/ai_generation.rs`
- `apps/desktop/src-tauri/src/services/multimodal_grading.rs`

**交付物**

1. 执行记录写入逻辑
2. 业务表与执行记录关联方案
3. 执行审计查询方式说明

**验收标准**

1. 聊天、沟通、评语、公告、增强批改都能查到 `execution_record`；
2. `search_summary_json / reasoning_summary` 有真实值；
3. 业务结果能反查具体执行来源。

---

## WP-AI-BIZ-004：生成型业务统一请求工厂与执行来源追踪

**目标**

统一家校沟通、学期评语、活动公告三条生成业务的请求构建方式。

**业务问题**

1. 各生成服务仍手工拼装请求；
2. 执行来源不统一；
3. 后续追踪和回归成本高。

**范围**

- ExecutionRequestFactory 扩展
- 生成型业务统一 request schema
- 生成结果执行来源关联

**涉及文件**

- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_request_factory.rs`
- `apps/desktop/src-tauri/src/services/ai_generation.rs`
- `apps/desktop/src/pages/StudentDetailPage.tsx`
- `apps/desktop/src/pages/SemesterCommentsPage.tsx`
- `apps/desktop/src/pages/ActivityAnnouncementsPage.tsx`

**验收标准**

1. 三条生成业务全部改用工厂方法；
2. 页面可展示执行来源字段；
3. 活动公告不被错误套用证据检索策略。

---

## WP-AI-BIZ-005：学期评语批量任务运行时化

**目标**

把批量评语生成从 `AsyncTask + for` 循环，升级为“任务 + 子执行聚合”模型。

**业务问题**

1. 当前只有粗粒度进度；
2. 失败项不可回放；
3. 没有子项级执行记录。

**范围**

- 学生级子执行
- 任务聚合进度
- 失败项重试与恢复

**涉及文件**

- `apps/desktop/src-tauri/src/services/ai_generation.rs`
- `apps/desktop/src-tauri/src/services/async_task.rs`
- `apps/desktop/src/pages/SemesterCommentsPage.tsx`

**验收标准**

1. 每个学生生成都有独立执行记录；
2. 批量任务支持子项级追踪；
3. 失败恢复不再只靠粗粒度 task checkpoint。

---

## WP-AI-BIZ-006：作业批改主链运行时化

**目标**

让运行时从“增强评分子步骤”升级为“批改主业务编排层”。

**业务问题**

1. 目前运行时只处理 enhanced grading 子步骤；
2. 主流程仍是旧批处理模型；
3. 真正多模态输入还未进入运行时。

**范围**

- 批改阶段拆分
- 资产级执行轨迹
- 步骤级进度
- 图像附件进入运行时

**涉及文件**

- `apps/desktop/src-tauri/src/commands/assignment_grading.rs`
- `apps/desktop/src-tauri/src/services/multimodal_grading.rs`
- `apps/desktop/src-tauri/src/services/assignment_grading.rs`
- `apps/desktop/src/pages/AssignmentGradingPage.tsx`

**验收标准**

1. 批改主链能展示步骤级进度；
2. 每个资产有独立执行记录；
3. 增强批改请求真正带上作业图像附件。

---

## WP-AI-BIZ-007：运行时业务事件补齐与前后端真实贯通

**目标**

把前端已经监听、命令层已经预留的事件类型，变成后端运行时真实产出的业务事件。

**业务问题**

1. 前端已监听 `ThinkingStatus / SearchSummary / Reasoning / ToolCall / ToolResult`；
2. 但运行时主链真实只稳定产出 `Start / Chunk / Complete / Error`；
3. 造成前后端能力看似齐全，实际空转。

**范围**

- ThinkingStatus 产出
- SearchSummary 产出
- Reasoning 产出
- ToolCall / ToolResult 的支持策略明确化

**涉及文件**

- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs`
- `apps/desktop/src-tauri/src/commands/execution.rs`
- `apps/desktop/src/hooks/useChatStream.ts`

**验收标准**

1. 前端已监听的关键事件至少有一条真实业务链稳定收到；
2. 工具事件要么真实支持，要么明确不支持并删除空预留；
3. 事件协议与实际行为一致。

---

## WP-AI-BIZ-008：OCR 真链路接入与模拟路径降级

**目标**

让 OCR 真正成为生产能力，而不是默认依赖模拟回退。

**业务问题**

1. 当前 ONNX Runtime 未接入；
2. OCR 模型缺位时默认模拟识别；
3. 批改链因此无法真正闭环。

**范围**

- OCR 后端方案接入
- 模拟路径降级为开发测试用途
- OCR 状态可视化与审计

**涉及文件**

- `apps/desktop/src-tauri/src/services/ocr.rs`
- `apps/desktop/src-tauri/src/commands/assignment_grading.rs`
- `apps/desktop/src/pages/AssignmentGradingPage.tsx`

**验收标准**

1. OCR 不再默认回退模拟识别；
2. 前端可区分真实 OCR 与模拟 OCR；
3. OCR 失败、回退、耗时可追踪。

---

## WP-AI-BIZ-009：练习卷生成运行时化评估与接入

**目标**

明确练习卷生成是否进入统一运行时，并据此落地。

**业务问题**

1. 当前完全是规则链；
2. 与批改/错题链路相邻，但不在统一 AI 体验里；
3. 后续容易被误判为“已经跟随 AI 重构升级”。

**范围**

- 规则层 / AI 层拆分
- 是否引入 profile
- 题目改写/变式/个性化接入运行时

**涉及文件**

- `apps/desktop/src-tauri/src/services/practice_sheet.rs`
- `apps/desktop/src/pages/PracticeSheetsPage.tsx`

**验收标准**

1. 给出明确结论：纳入或暂不纳入；
2. 若纳入，至少有一段 AI 题目生成/改写走统一运行时；
3. 若不纳入，文档和架构说明明确标注其边界。

---

## WP-AI-BIZ-010：MCP / Skill 业务默认接入

**目标**

让 MCP / Skill 从“底座可用”变成“业务默认可用”。

**业务问题**

1. MCP 构建器支持但业务 builder 未注入；
2. Skill 有独立执行器，但不是统一运行时默认路径；
3. 工具面和 profile 权限没有完全统一。

**范围**

- MCP 服务器注入业务 builder
- Skill 暴露到运行时工具面
- 工具权限、健康状态、事件审计统一

**涉及文件**

- `apps/desktop/src-tauri/src/commands/execution.rs`
- `apps/desktop/src-tauri/src/services/ai_generation.rs`
- `apps/desktop/src-tauri/src/services/multimodal_grading.rs`
- `apps/desktop/src-tauri/src/services/ai_orchestration/tool_exposure.rs`
- `apps/desktop/src-tauri/src/services/skill_tool_adapter.rs`

**验收标准**

1. 至少一个真实业务场景可控使用 MCP 工具；
2. 技能调用事件可见、可审计；
3. 工具暴露与 profile 权限真正一致。

---

## 4. 最小执行顺序

如果只按“最小可落地路径”推进，建议顺序如下：

1. `WP-AI-BIZ-001`
2. `WP-AI-BIZ-002`
3. `WP-AI-BIZ-007`
4. `WP-AI-BIZ-003`
5. `WP-AI-BIZ-004`
6. `WP-AI-BIZ-005`
7. `WP-AI-BIZ-006`
8. `WP-AI-BIZ-008`
9. `WP-AI-BIZ-009`
10. `WP-AI-BIZ-010`

原因很简单：

1. 先把聊天与搜索主链补齐；
2. 再补统一执行账本；
3. 再统一生成型业务；
4. 最后收拢批改、OCR、练习卷、MCP/Skill。

---

## 5. 建议的立项方式

建议每个工作包都使用单独编号立项，并带上以下字段：

1. 背景问题
2. 业务目标
3. 范围边界
4. 涉及文件
5. 验收标准
6. 风险与回滚策略

如果需要压缩为更少项目，可以合并为 4 个大项：

1. **运行时主链闭环包**：001 + 002 + 003 + 007
2. **生成型业务统一包**：004 + 005
3. **批改/OCR 闭环包**：006 + 008
4. **扩展能力接入包**：009 + 010

---

## 6. 最终说明

这份工作包文档的目的不是重复原修复计划，而是把原计划进一步压缩成：

> **可以直接排期、直接分配、直接执行的工作单元。**

使用时建议：

1. 先按批次立项；
2. 再把单个工作包展开成开发任务；
3. 每完成一个工作包就做一次业务回归，而不是只看代码是否“已经重构”。

# PureWorker 技术方案（v1.0 / 对齐 PRD v1.0）

> 文档目标：将 `doc/prd-v1.0.md` 转化为可实施、可排期、可验收的工程方案。  
> 适用范围：一期 MVP（PRD 第 7 章）——系统设置（5）、基础教务管理（3）、作业/考评与题库（4.2）、班务与家校沟通（4.3）。

---

## 1. 方案结论（先给结论）

采用 **Tauri（Rust Core + React/TypeScript UI）+ 本地优先数据架构**。

- **桌面框架**：Tauri 2.x（Windows/macOS 双端）
- **前端**：React + TypeScript + Zustand + TanStack Query
- **Agent 分层（MVP）**：Rust 后端 Rig 统一承担 Agent 编排 + LLM 接入 + Tool 调用，前端仅负责渲染与交互
- **本地结构化数据**：SQLite
- **本地对象存储**：文件系统分区目录（原始素材、中间产物、导出成品）
- **记忆引擎（MVP）**：
  - 结构化记忆：SQLite（学生档案、成绩、标签、沟通记录）
  - 文本检索：SQLite FTS（观察记录、历史评语、沟通文本）
  - 长期记忆：学生/班级目录下 Markdown 记忆文件
  - 检索方式：Agentic Search（SQL 过滤 + 文件遍历 + 规则重排）
- **AI 调用层**：Rust Rig Provider 适配层（OpenAI/Anthropic/阿里/DeepSeek 等）
- **文档处理层（Skills）**：内置技能运行时（OCR、文档读写、表格处理、模板套用）
- **MCP 扩展层（MVP）**：受控接入（stdio 优先，按白名单启用）
- **安全策略**：最小权限、默认本地存储、敏感信息系统密钥链托管、所有输出“教师确认后生效”

该方案满足 PRD 的三项核心：
1) 本地文件自动化处理；2) 长效记忆（SQLite+Markdown 文件化记忆）；3) 可扩展能力（MVP 先内置，生态化后置）。

---

## 2. 一期 MVP 边界与不做项

## 2.1 MVP 必做（与 PRD 第 7 章完全对齐）

1. **系统设置与个性化（模块 5 全量）**
   - AI 供应商与密钥配置
   - 本地存储路径与数据生命周期（导出/归档/擦除）
   - 教师身份预设、语气库、评价体系
   - 文档模板管理、默认导出偏好
   - 全局快捷键、后台监控接收文件夹
   - Skills 与 MCP 集成基础能力
   - 快捷指令 `/` 配置

2. **基础教务管理（模块 3 全量）**
   - 班级/学科管理
   - 学生 360 档案（Excel 导入、标签、成长轨迹）
   - 日程与课表管理，教案/课件与课次关联

3. **作业/考评与题库（模块 4.2 全量）**
   - 非标准作业批量结构化（图片 OCR + 汇总 Excel）
   - 针对性错题重组与专属练习（按学生历史错题生成）

4. **班务与家校沟通（模块 4.3 全量）**
   - 家长沟通文案（带记忆注入）
   - 期末评语批量生成（可追溯依据）
   - 班会/校园活动文案策划

## 2.2 MVP 暂不做（P1.5/P2）

- 多源素材一键转 PPT（4.1）
- 音视频教研“榨汁机”（4.1，ASR 大规模任务）


> 说明：不做项不影响一期主价值闭环（“导入素材 → AI 处理 → 人工确认 → 导出成品”）。

---

## 3. 总体架构设计

## 3.1 分层架构

```text
┌────────────────────────────┐
│ UI 层 (React + TS)          │  仅渲染与交互：三栏布局/任务面板/结果卡片/设置中心
└──────────────┬─────────────┘
               │ Tauri IPC
┌──────────────▼─────────────┐
│ Rust Agent Core (Rig)       │  任务编排、状态机、白盒进度、人工确认
└───────┬──────────┬─────────┘
        │          │
┌───────▼───┐  ┌───▼────────┐
│ LLM/Tool   │  │ Skills 运行时│  内置技能 + 外部 MCP 工具
│ Rig Adapter│  │ 文档/OCR/导出 │
└───────┬───┘  └───┬────────┘
        │          │
┌───────▼──────────▼─────────┐
│ 数据层                       │
│ SQLite(结构化+FTS) + 文件夹记忆库 + Agentic Search │
└────────────────────────────┘
```

## 3.2 关键设计原则

1. **本地优先（Local-first）**：学生数据、作业图片、评语草稿默认本地落盘。
2. **结果导向**：所有工作流以“可导出的成品”作为完成标准。
3. **白盒执行**：步骤级进度可视化（已完成/进行中/等待中）。
4. **人审闭环**：输出默认草稿，需教师确认后才可采纳/导出。
5. **可扩展**：MVP 先保证内置能力闭环，外部生态按安全策略渐进开放。
6. **前后端职责分离**：前端不承担模型调用与编排逻辑，统一由 Rust 后端执行。
7. **高危确认**：外发、覆盖写入、批量变更必须人工确认。
8. **禁删红线**：AI/Skills/MCP 默认不具备删除文件能力。

---

## 4. 模块级技术方案（MVP）

## 4.1 模块 5：系统设置与个性化（中枢）

### 4.1.1 子模块
- AI 配置中心：Provider、模型、语义化参数（严谨/创意）
- 安全与隐私：存储目录、脱敏开关、数据打包导出、安全擦除
- 教师身份与语气库：学段/学科/教材版本、写作风格样本
- 校本模板：红头模板上传、模板版本控制、默认导出格式/路径
- 全局行为：快捷键、后台监控文件夹
- 扩展中心：Skills 安装、MCP Server 接入
- 运行时体检中心：Python/uv 状态检测、版本展示、一键修复入口

### 4.1.2 实现要点
- 配置源分级：`system defaults < school policy < teacher preference`
- 密钥存储：OS Keychain（Win Credential Manager / macOS Keychain）
- 模板存储：`/templates/{school}/{type}/{version}` + 元数据表
- 记忆目录：`/workspace/classes/{class_id}/`、`/workspace/students/{student_id}/memory/*.md`

#### 4.1.2.1 uv 运行时健康检查（设置页）

- 页面进入设置-扩展中心时自动执行健康检查：
  1) PATH 检测：`uv --version`
  2) 回退检测：`~/.local/bin/uv`（macOS）或 `%USERPROFILE%\\.local\\bin\\uv.exe`（Windows）
  3) 结果校验：以绝对路径再次执行 `--version`，防止坏链路或伪命令
- 状态展示：
  - `已就绪`（显示版本）
  - `未安装`
  - `版本过低`
  - `检测失败`（可查看错误详情）
- 操作入口：
  - 按钮：`一键安装/修复 uv`
  - 按钮：`重新检测`
  - 链接：`离线安装指南`（校园网限制场景）

> 说明：一键安装必须由用户主动触发，不允许静默安装。

## 4.2 模块 3：基础教务管理（数据基座）

### 4.2.1 核心能力
- Excel 批量导入学生名单（模板校验 + 错误行反馈）
- 班级/学科/教师关系建模
- 学生画像：标签、成绩时间序列、观察记录、沟通历史
- 日程课表：课次与教案/课件文件关联

### 4.2.2 关键约束
- 一切 AI 生成内容必须可追溯到学生档案依据。
- 学生档案数据变更要有审计日志（谁/何时/改了什么）。

## 4.3 模块 4.2：作业/考评与题库

### 4.3.1 非标准作业批量结构化
流程：
1) 拖入多张作业图片  
2) 预处理（去噪/矫正/分割）  
3) OCR 提取 + 题块定位（基础模式）  
4) 多模态批卷（增强模式，可选）：上传学生试卷 + 标准答案后进行题目级判定  
5) 结果融合：OCR结构化结果与多模态判定结果做一致性校验与冲突标记  
6) 按学号姓名归并  
7) 自动核对（规则+模型）  
8) 输出结构化 Excel（姓名/学号/题号/得分/置信度/冲突标记）

Tauri 文件拖拽实现约束（必须）：
- 前端拖拽后**只上传文件路径/句柄元数据**，禁止将图片转 Base64 后通过 IPC 传输。
- Rust 后端根据白名单路径自行读取文件并处理，避免 IPC 大包序列化导致内存飙升。
- 单次任务仅传输轻量参数（job_id、path 列表、配置项），二进制数据留在本地文件系统。

### 4.3.2 错题重组与专属练习
流程：
1) 从学生历史错题库拉取近一月错题  
2) 题目模板化（知识点、难度、题型）  
3) 参数扰动生成同类题  
4) 生成 Word 练习卷 + 答案页

### 4.3.3 技术策略
- 双模式批卷：
  - 基础模式：OCR 负责信息提取、题块定位、文本结构化
  - 增强模式：多模态 LLM 用于“学生试卷 + 标准答案”的题目级批卷判定
  - 降级策略：当多模态 LLM 未配置、不可用或超时时，自动回落 OCR + 规则判分
- OCR 引擎实现：
  - 采用 **ONNX Runtime + PaddleOCR 模型** 方案
  - 原因：跨平台一致（Windows/macOS）、便于 Rust 后端统一封装调用
- 判分：先规则后模型；以教师提供答案键/评分规则为准；低置信度自动进入人工确认
- 结果落地：导出前提供逐条复核界面

## 4.4 模块 4.3：班务与家校沟通

### 4.4.1 家长沟通文案
- Prompt 注入来源：近期成绩趋势 + 标签 + 观察记录 + 历史沟通语气
- 输出结构：先肯定、再反馈问题、再给可执行建议
- 卡片操作：编辑 / 采纳 / 重新生成

### 4.4.2 期末评语批量生成
- 批处理队列（班级维度）
- 反重复机制：语义去重 + 模板多样化
- 依据标注：每条评语附数据来源计数

### 4.4.3 班会/活动全案
- 主题输入 → 多对象文案（家长版/学生版/校内通知版）
- 支持学校模板自动套用与导出

---

## 5. 数据模型设计（核心表）

## 5.1 结构化模型（SQLite）

- `teacher_profile(id, name, stage, subject, textbook_version, tone_preset, created_at)`
- `classroom(id, grade, class_name, subject, teacher_id)`
- `student(id, student_no, name, gender, class_id, meta_json)`
- `student_tag(id, student_id, tag_name, created_at, is_deleted)`
- `score_record(id, student_id, exam_name, subject, score, full_score, exam_date)`
- `observation_note(id, student_id, content, source, created_at)`
- `parent_communication(id, student_id, draft, adopted_text, status, evidence_json, created_at)`
- `async_task(id, task_type, target_id, status, progress_json, context_data, checkpoint_cursor, completed_items_json, partial_output_path, lease_until, attempt_count, created_at, updated_at)`
- `assignment_asset(id, file_path, hash, class_id, captured_at)`
- `assignment_ocr_result(id, asset_id, student_id, question_no, answer_text, confidence, score)`
- `question_bank(id, source, knowledge_point, difficulty, stem, answer, explanation)`
- `schedule_event(id, class_id, title, start_at, end_at, linked_file_id)`
- `template_file(id, type, school_scope, version, file_path, enabled)`
- `skill_registry(id, name, version, source, permission_scope, status)`
- `mcp_server_registry(id, name, transport, command, args_json, env_json, permission_scope, enabled)`
- `audit_log(id, actor, action, target_type, target_id, diff_json, created_at)`

## 5.2 Agentic Search 记忆检索（无向量检索）

- 目录组织：
  - 班级目录：`/workspace/classes/{class_id}/`
  - 学生目录：`/workspace/students/{student_id}/`
  - 记忆文件：`/workspace/students/{student_id}/memory/*.md`
  - 作业/试卷：`/workspace/students/{student_id}/assignments/*`、`/workspace/students/{student_id}/exams/*`
- 检索流程：
  1) SQL 精确过滤（学生/班级/时间/标签）
  2) SQLite FTS 召回（观察、评语、沟通）
  3) 文件遍历补充（Markdown 记忆、错题与素材文件）
  4) 规则重排（近期优先、学科优先、高置信优先）
- 注入策略：仅注入 Top-K 证据，避免上下文污染。

#### 5.2.1 学生长期记忆 Markdown 标准格式（1.0）

> 每名学生按月滚动维护记忆文件：`/workspace/students/{student_id}/memory/{YYYY-MM}.md`

文件头（YAML Frontmatter）：
```yaml
student_id: S001
student_name: 张三
class_id: C001
homeroom_teacher_id: T001
version: 1.0
last_updated_at: 2026-03-07T18:30:00+08:00
```

正文分区（固定章节，便于稳定解析）：
```markdown
## 学习表现观察
- [2026-03-02][math][课堂表现] 空间想象偏弱，立体几何审题慢。

## 错题与薄弱点
- [2026-03-03][math][立体几何] 高风险错因：辅助线构造不稳定。

## 家校沟通纪要
- [2026-03-05][家校沟通] 家长反馈作业时长增加，情绪焦虑。

## 干预策略与效果
- [2026-03-06][策略] 每周2题分层训练；[效果] 正确率由40%提升到60%。

## 评语素材池
- [关键词:进步明显] 本月课堂参与度提升，表达更主动。
```

格式约束：
- 每条记忆必须带日期标签 `[YYYY-MM-DD]`
- 建议带学科标签（如 `[math]`）与类型标签（如 `[课堂表现]`）
- 禁止写入学生身份证号、家庭住址等高敏信息原文

#### 5.2.2 学生记忆读取工具定义（Rust 封装）

1) `memory.read_student_memory_timeline`
- 中文名称：**学生记忆时间线读取工具**
- 说明：按时间窗口读取学生长期记忆条目，返回标准化时间线。
- 输入：`student_id`, `from_date`, `to_date`, `section_filter[]`, `limit`
- 输出：`entries[]`（time, section, tags, content, source_file）
- 实现约束：通过 Rust `MemoryService` 读取并解析 Markdown，禁止 shell 命令。

2) `memory.read_student_memory_by_topic`
- 中文名称：**学生专题记忆读取工具**
- 说明：按学科/主题读取相关记忆（如“立体几何”“作业习惯”）。
- 输入：`student_id`, `topic`, `subject`, `top_k`
- 输出：`topic_entries[]`（evidence, confidence, source_ref）
- 实现约束：先用 SQLite 索引过滤，再由 Rust 文件接口读取命中文件片段。

3) `memory.read_student_comment_materials`
- 中文名称：**评语素材读取工具**
- 说明：提取“评语素材池”章节，用于期末评语批量生成。
- 输入：`student_id`, `term`, `subject`
- 输出：`materials[]`（positive_points, risks, suggestions）
- 实现约束：仅返回脱敏后的可生成素材。

Markdown 记忆模板：
```markdown
---
memory_type: observation | communication | error_pattern
student_id: S001
class_id: C001
subject: math
created_at: 2026-03-07T10:00:00+08:00
tags: [课堂表现, 几何薄弱]
source: teacher_note
---

### 关键观察
- 立体几何错因：空间想象不足
- 建议：每周 2 题分层训练
```

## 5.3 SQLite 数据库设计（含中文注释）

> 说明：一期以 SQLite 为唯一结构化存储；长期记忆正文落 Markdown 文件，数据库保存索引与映射。

#### 5.3.1 SQLite 连接初始化参数（并发强制要求）

> 必须在 Rust 建立连接池（如 `sqlx`）时执行，用于前台读 + 后台批量写并发场景。

```sql
PRAGMA journal_mode=WAL;       -- 开启预写日志，允许读写并发
PRAGMA synchronous=NORMAL;     -- 平衡安全与性能（WAL 推荐）
PRAGMA busy_timeout=5000;      -- 高并发写入时避免直接报错
PRAGMA foreign_keys=ON;        -- 强制外键约束（每个连接都要开启）
```

补充约束：
- 连接池初始化失败应阻断任务启动，避免以默认模式运行。
- 后台批量任务（OCR/批卷/生成）建议使用事务分批提交，减少锁持有时间。
- `foreign_keys=ON` 对连接池是“逐连接生效”，必须在 Rust 连接创建钩子中设置，不可只在迁移里设置一次。

```sql
-- 教师档案表：存储教师身份预设信息
CREATE TABLE teacher_profile (
  id TEXT PRIMARY KEY,                 -- 教师ID
  name TEXT NOT NULL,                  -- 姓名
  stage TEXT NOT NULL,                 -- 学段
  subject TEXT NOT NULL,               -- 学科
  textbook_version TEXT,               -- 教材版本
  tone_preset TEXT,                    -- 语气预设
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- 班级表：绑定班级与教师
CREATE TABLE classroom (
  id TEXT PRIMARY KEY,                 -- 班级ID
  grade TEXT NOT NULL,                 -- 年级
  class_name TEXT NOT NULL,            -- 班级名
  subject TEXT NOT NULL,               -- 学科
  teacher_id TEXT NOT NULL,            -- 班主任/任课教师ID
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (teacher_id) REFERENCES teacher_profile(id)
);

-- 学生基本信息表：存储学生主数据
CREATE TABLE student (
  id TEXT PRIMARY KEY,                -- 学生ID（UUID）
  student_no TEXT NOT NULL,           -- 学号
  name TEXT NOT NULL,                 -- 姓名
  gender TEXT,                        -- 性别
  class_id TEXT NOT NULL,             -- 所属班级ID
  meta_json TEXT,                     -- 扩展元数据
  folder_path TEXT NOT NULL,          -- 学生目录绝对路径
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,           -- 创建时间
  updated_at TEXT NOT NULL,           -- 更新时间
  FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- 学生标签表：标准化标签映射
CREATE TABLE student_tag (
  id TEXT PRIMARY KEY,
  student_id TEXT NOT NULL,
  tag_name TEXT NOT NULL,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  FOREIGN KEY (student_id) REFERENCES student(id)
);

-- 对话会话表：存储用户与AI会话元数据
CREATE TABLE conversation (
  id TEXT PRIMARY KEY,                -- 会话ID
  teacher_id TEXT NOT NULL,           -- 教师ID
  title TEXT,                         -- 会话标题
  scenario TEXT,                      -- 场景（评语/沟通/作业等）
  created_at TEXT NOT NULL,           -- 创建时间
  updated_at TEXT NOT NULL,           -- 更新时间
  FOREIGN KEY (teacher_id) REFERENCES teacher_profile(id)
);

-- 对话消息表：存储会话消息明细
CREATE TABLE conversation_message (
  id TEXT PRIMARY KEY,                -- 消息ID
  conversation_id TEXT NOT NULL,      -- 会话ID
  role TEXT NOT NULL,                 -- 角色（user/assistant/system/tool）
  content TEXT NOT NULL,              -- 消息内容
  tool_name TEXT,                     -- 调用工具名（可空）
  created_at TEXT NOT NULL,           -- 发送时间
  FOREIGN KEY (conversation_id) REFERENCES conversation(id)
);

-- 学生文件映射表：存储学生文件索引与类型
CREATE TABLE student_file_map (
  id TEXT PRIMARY KEY,                -- 映射ID
  student_id TEXT NOT NULL,           -- 学生ID
  file_type TEXT NOT NULL,            -- 文件类型（assignment/exam/memory/doc）
  file_path TEXT NOT NULL,            -- 文件绝对路径
  file_hash TEXT,                     -- 文件哈希（防重复）
  source TEXT,                        -- 来源（上传/导入/生成）
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,           -- 建立时间
  FOREIGN KEY (student_id) REFERENCES student(id)
);

-- Markdown记忆索引表：存储记忆文件元数据，正文在文件中
CREATE TABLE memory_index (
  id TEXT PRIMARY KEY,                -- 索引ID
  student_id TEXT NOT NULL,           -- 学生ID
  class_id TEXT NOT NULL,             -- 班级ID
  memory_type TEXT NOT NULL,          -- 记忆类型
  file_path TEXT NOT NULL,            -- Markdown文件路径
  summary TEXT,                       -- 记忆摘要
  created_at TEXT NOT NULL,           -- 创建时间
  is_deleted INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (student_id) REFERENCES student(id),
  FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- 观察记录表：作为 FTS 检索的数据源
CREATE TABLE observation_note (
  id TEXT PRIMARY KEY,
  student_id TEXT NOT NULL,
  content TEXT NOT NULL,
  source TEXT,
  created_at TEXT NOT NULL,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (student_id) REFERENCES student(id)
);

-- 家校沟通表：作为 FTS 检索的数据源
CREATE TABLE parent_communication (
  id TEXT PRIMARY KEY,
  student_id TEXT NOT NULL,
  draft TEXT,
  adopted_text TEXT,
  status TEXT,
  evidence_json TEXT,
  created_at TEXT NOT NULL,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (student_id) REFERENCES student(id)
);

-- 成绩统计表：存储统计分析需要的结构化分数数据
CREATE TABLE score_record (
  id TEXT PRIMARY KEY,                -- 记录ID
  student_id TEXT NOT NULL,           -- 学生ID
  exam_name TEXT NOT NULL,            -- 考试名称
  subject TEXT NOT NULL,              -- 学科
  score REAL NOT NULL,                -- 得分
  full_score REAL NOT NULL,           -- 满分
  rank_in_class INTEGER,              -- 班级排名
  exam_date TEXT NOT NULL,            -- 考试日期
  is_deleted INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (student_id) REFERENCES student(id)
);

-- 异步任务队列表：统一长任务状态与断点续跑
CREATE TABLE async_task (
  id TEXT PRIMARY KEY,
  task_type TEXT NOT NULL,            -- 任务类型（batch_ocr/generate_comments/...）
  target_id TEXT,                     -- 关联实体ID（班级/学生）
  status TEXT NOT NULL,               -- queued/running/waiting_human/recovering/completed/failed/cancelled
  progress_json TEXT,                 -- 进度详情
  context_data TEXT,                  -- 输入上下文
  checkpoint_cursor TEXT,
  completed_items_json TEXT,
  partial_output_path TEXT,
  lease_until TEXT,                   -- 租约截止（防并发抢占）
  attempt_count INTEGER NOT NULL DEFAULT 0,
  last_heartbeat_at TEXT,              -- 心跳时间
  worker_id TEXT,                      -- 工作线程标识
  error_code TEXT,                     -- 错误码
  error_message TEXT,                  -- 错误详情
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- 作业素材表
CREATE TABLE assignment_asset (
  id TEXT PRIMARY KEY,
  class_id TEXT NOT NULL,
  file_path TEXT NOT NULL,
  hash TEXT,
  captured_at TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- 作业OCR结果表
CREATE TABLE assignment_ocr_result (
  id TEXT PRIMARY KEY,
  asset_id TEXT NOT NULL,
  job_id TEXT,                         -- 批次任务ID（关联 async_task）
  student_id TEXT NOT NULL,
  question_no TEXT,
  answer_text TEXT,
  confidence REAL,
  score REAL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (asset_id) REFERENCES assignment_asset(id),
  FOREIGN KEY (student_id) REFERENCES student(id)
);

-- 题库表
CREATE TABLE question_bank (
  id TEXT PRIMARY KEY,
  source TEXT,
  knowledge_point TEXT,
  difficulty TEXT,
  stem TEXT NOT NULL,
  answer TEXT,
  explanation TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- 课表/日程表
CREATE TABLE schedule_event (
  id TEXT PRIMARY KEY,
  class_id TEXT NOT NULL,
  title TEXT NOT NULL,
  start_at TEXT NOT NULL,
  end_at TEXT,
  linked_file_id TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- 模板文件表
CREATE TABLE template_file (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  school_scope TEXT,
  version TEXT,
  file_path TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

-- Skills 注册表
CREATE TABLE skill_registry (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  version TEXT,
  source TEXT,
  permission_scope TEXT,
  status TEXT,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

-- MCP Server 注册表
CREATE TABLE mcp_server_registry (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  transport TEXT NOT NULL,
  command TEXT,
  args_json TEXT,
  env_json TEXT,
  permission_scope TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  is_deleted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

-- 审计日志表：记录高风险操作与确认信息
CREATE TABLE audit_log (
  id TEXT PRIMARY KEY,                -- 日志ID
  actor TEXT NOT NULL,                -- 操作者（teacher/ai/system）
  action TEXT NOT NULL,               -- 动作（export/overwrite/tool_call等）
  target_type TEXT NOT NULL,          -- 目标类型（file/message/student）
  target_id TEXT,                     -- 目标ID或路径
  risk_level TEXT NOT NULL,           -- 风险等级（low/medium/high/critical）
  confirmed_by_user INTEGER NOT NULL, -- 是否经用户确认（0/1）
  created_at TEXT NOT NULL            -- 时间
);

-- 审批请求持久化表：人工确认闸门的 source of truth
CREATE TABLE approval_request (
  id TEXT PRIMARY KEY,                 -- 请求ID
  task_id TEXT,                        -- 关联异步任务ID（可空，部分审批无任务上下文）
  request_type TEXT NOT NULL,          -- 审批类型（export/overwrite/batch_send等）
  action_summary TEXT NOT NULL,        -- 操作摘要（展示给教师）
  params_preview TEXT,                 -- 参数预览 JSON（前端展示用）
  risk_level TEXT NOT NULL,            -- 风险等级（low/medium/high/critical）
  status TEXT NOT NULL DEFAULT 'pending', -- pending/approved/rejected/expired
  resolved_by TEXT,                    -- 处理人（teacher/system/timeout）
  resolved_at TEXT,                    -- 处理时间
  timeout_at TEXT NOT NULL,            -- 超时截止时间
  created_at TEXT NOT NULL,            -- 创建时间
  FOREIGN KEY (task_id) REFERENCES async_task(id)
);
CREATE INDEX IF NOT EXISTS idx_approval_pending ON approval_request(status, timeout_at) WHERE status='pending';

-- FTS5 全文检索表：采用独立投影，避免 TEXT 主键外部内容映射陷阱
CREATE VIRTUAL TABLE memory_fts USING fts5(
  source_table,
  source_id,
  student_id,
  class_id,
  content,
  created_at,
  -- ⚠️ 中文分词策略说明：
  -- unicode61 对中文按字符逐字切分（非词级），可支持子串匹配但无法实现语义词级检索。
  -- MVP 阶段可接受：配合 LIKE 降级查询覆盖短词场景。
  -- Phase-2 升级路径：编译 jieba-fts5 tokenizer 为 SQLite loadable extension，
  -- 届时仅需修改此行为 tokenize = 'jieba' 并重建 FTS 索引（ALTER 不支持，需 DROP+CREATE）。
  tokenize = 'unicode61'
);

-- FTS 同步触发器（observation_note）
CREATE TRIGGER IF NOT EXISTS obs_note_ai AFTER INSERT ON observation_note
WHEN new.is_deleted=0
BEGIN
  INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
  VALUES('observation_note', new.id, new.student_id, coalesce((SELECT class_id FROM student WHERE id=new.student_id), ''), new.content, new.created_at);
END;

CREATE TRIGGER IF NOT EXISTS obs_note_au AFTER UPDATE ON observation_note BEGIN
  -- 先删除旧 FTS 记录
  DELETE FROM memory_fts WHERE source_table='observation_note' AND source_id=old.id;
  -- 仅当未软删除时重新插入（软删除时仅保留 DELETE 效果）
  INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
  SELECT 'observation_note', new.id, new.student_id, coalesce((SELECT class_id FROM student WHERE id=new.student_id), ''), new.content, new.created_at
  WHERE new.is_deleted=0;
END;

CREATE TRIGGER IF NOT EXISTS obs_note_ad AFTER DELETE ON observation_note BEGIN
  DELETE FROM memory_fts WHERE source_table='observation_note' AND source_id=old.id;
END;

-- FTS 同步触发器（parent_communication）
CREATE TRIGGER IF NOT EXISTS pc_ai AFTER INSERT ON parent_communication
WHEN new.is_deleted=0
BEGIN
  INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
  VALUES('parent_communication', new.id, new.student_id, coalesce((SELECT class_id FROM student WHERE id=new.student_id), ''), coalesce(new.adopted_text, new.draft), new.created_at);
END;

CREATE TRIGGER IF NOT EXISTS pc_au AFTER UPDATE ON parent_communication BEGIN
  -- 先删除旧 FTS 记录
  DELETE FROM memory_fts WHERE source_table='parent_communication' AND source_id=old.id;
  -- 仅当未软删除时重新插入
  INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
  SELECT 'parent_communication', new.id, new.student_id, coalesce((SELECT class_id FROM student WHERE id=new.student_id), ''), coalesce(new.adopted_text, new.draft), new.created_at
  WHERE new.is_deleted=0;
END;

CREATE TRIGGER IF NOT EXISTS pc_ad AFTER DELETE ON parent_communication BEGIN
  DELETE FROM memory_fts WHERE source_table='parent_communication' AND source_id=old.id;
END;

-- 活跃数据索引（软删除过滤）
CREATE INDEX IF NOT EXISTS idx_student_active_class ON student(class_id) WHERE is_deleted=0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_student_tag_unique_active ON student_tag(student_id, tag_name) WHERE is_deleted=0;
CREATE INDEX IF NOT EXISTS idx_memory_index_active ON memory_index(student_id, class_id, created_at) WHERE is_deleted=0;
CREATE INDEX IF NOT EXISTS idx_async_task_claim ON async_task(status, lease_until, updated_at);
CREATE INDEX IF NOT EXISTS idx_student_no_class ON student(student_no, class_id) WHERE is_deleted=0;
CREATE INDEX IF NOT EXISTS idx_score_student_subject ON score_record(student_id, subject, exam_date);
CREATE INDEX IF NOT EXISTS idx_score_exam ON score_record(student_id, exam_name, subject);
CREATE INDEX IF NOT EXISTS idx_obs_note_student ON observation_note(student_id, created_at) WHERE is_deleted=0;
CREATE INDEX IF NOT EXISTS idx_pc_student ON parent_communication(student_id, created_at) WHERE is_deleted=0;
CREATE INDEX IF NOT EXISTS idx_ocr_result ON assignment_ocr_result(asset_id, student_id, question_no);
CREATE INDEX IF NOT EXISTS idx_conv_msg ON conversation_message(conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_file_map_student ON student_file_map(student_id, file_type, created_at) WHERE is_deleted=0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_template_unique ON template_file(type, school_scope, version) WHERE is_deleted=0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_skill_unique ON skill_registry(name, version) WHERE is_deleted=0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_mcp_unique ON mcp_server_registry(name) WHERE is_deleted=0;
CREATE INDEX IF NOT EXISTS idx_approval_task ON approval_request(task_id) WHERE status='pending';

-- 查询约束：所有业务查询默认追加 is_deleted=0（软删除行不可见）
```

---

## 6. 工作流与状态机设计

统一任务状态：`queued → running → waiting_human → completed / failed / cancelled`

每个工作流都输出：
1) 人类可读步骤日志；2) 当前进度百分比；3) 预计剩余时间；4) 可中止。

需要人工介入时切换 `waiting_human`，由 UI 显示“确认卡片”。

### 6.1 长耗时任务持久化与崩溃恢复（工程补丁）

> 适用于“期末评语批量生成”“批量作业结构化”等 5-10 分钟级任务，防止进程退出导致任务结果丢失。

实现要求（必须）：
1. 任务创建即持久化：`task_id`、任务类型、输入快照、当前进度、已完成分片。
2. 分片提交策略：每完成一个学生/一组作业即落盘中间结果（不得仅存内存）。
3. 可恢复状态机：新增 `recovering` 状态，启动时扫描 `running/recovering` 任务并执行恢复。
4. 幂等恢复：按 `task_id + item_id` 去重，已完成分片不得重复写入。
5. 用户可见恢复：启动后展示“检测到未完成任务，是否继续/终止”。
6. 任务领取采用“租约”语义：通过 `lease_until` 防止多 worker 重复领取同一任务。

推荐补充状态：
`queued → running → waiting_human → recovering → completed / failed / cancelled`

建议持久化字段：
- `checkpoint_cursor`（当前分片）
- `completed_items_json`（已完成条目）
- `partial_output_path`（中间产物路径）
- `last_heartbeat_at`（心跳时间）
- `lease_until`（任务租约截止时间）

## 6.2 人工确认闸门的异步挂起机制（工程补丁）

> 适用于 `approval.request_user_confirmation`。目标：等待教师确认时不阻塞 Tokio 运行时与并发任务队列。

**持久化层（source of truth）**：所有审批请求持久化到 `approval_request` 表，确保进程崩溃/重启后可恢复未决审批状态。`oneshot` 通道仅作为进程内唤醒机制，不承担状态存储职责。

实现要求（必须）：
1. 后端触发确认时生成唯一 `request_id`，写入 `approval_request` 表（status=pending），同时创建 `tokio::sync::oneshot` 通道。
2. 将 `request_id -> oneshot::Sender` 放入待处理映射（如 `DashMap` / async mutex map）。
3. 通过 Tauri Event 将确认请求推送前端（含 `request_id`、操作摘要、风险级别、参数预览）。
4. 工具函数内部 `await oneshot::Receiver`，交出线程控制权（异步挂起）。
5. 前端点击后，通过 Tauri Command `resolve_confirmation(request_id, result)` 更新 `approval_request` 表状态并唤醒等待任务。
6. 进程启动时扫描 `approval_request` 表中 `status='pending'` 且未超时的记录，恢复对应审批流程。
7. 必须配置超时与清理：超时时更新 `approval_request.status='expired'`，移除待处理映射，防止通道泄漏与僵尸请求。

失败与边界处理：
- `request_id` 不存在：返回“请求已失效/已超时”。
- 通道被关闭（进程重启）：从 `approval_request` 表恢复状态，重建通道或直接标记 `expired`。
- 用户拒绝：更新 `approval_request.status='rejected'`，返回 `rejected`，流程进入安全回退分支。

---

## 7. Skills 与 MCP 集成方案（PRD 5.6）

## 7.1 Built-in Skills（内置动作引擎）

首批内置：
- `office.read_write`（docx/xlsx）
- `ocr.extract`（图片文字提取）
- `image.preprocess`（裁剪/矫正）
- `math.compute`（高精度计算）
- `export.render`（Word/Excel/PDF 导出）

Skill 声明结构：
- `name/version`
- `input_schema/output_schema`
- `permission_scope`（fs/net/process）
- `timeout/retry`

Claude Skills 兼容方案：
- 兼容 Agent Skills 目录规范：每个技能目录必须包含 `SKILL.md`
- `SKILL.md` 结构：YAML frontmatter + Markdown body
- frontmatter 必填：`name`、`description`
- 可选字段：`license`、`compatibility`、`metadata`、`allowed-tools`
- 建议目录：`scripts/`、`references/`、`assets/`
- 渐进加载（Progressive Disclosure）：目录摘要 → SKILL.md 全文 → 按需资源
- 扫描路径：`./.agents/skills/`、`~/.agents/skills/`，项目级覆盖用户级

## 7.1.1 Python Skills 运行环境策略（MVP）

- 目标：当 Skills 依赖 Python 时，做到“可运行、可隔离、可审计、可复现”。
- 内置优先原则（教师友好）：
  - **系统内置 Skills 随安装包预置 uv 运行时与依赖**，开箱即用，不要求教师手工配置 Python。
  - 第三方/自定义 Skills 才进入“检测 + 一键安装/修复”流程。
- 运行方式：
  1) 每个技能使用独立虚拟环境：`~/.pureworker/skill-envs/{skill_name}/{env_hash}/`
  2) 使用 `uv` 管理 Python 与依赖（优先），避免污染系统 Python
  3) 子进程执行脚本（禁交互、超时、资源限制）
- 依赖声明约定（兼容 Agent Skills）：
  - 在 `SKILL.md` 的 `compatibility` / `metadata` 中声明 Python 版本与依赖文件
  - 推荐提供 `requirements.txt` 或锁定文件（带版本）
- 安装策略：
  - 内置 Skills：安装阶段直接落盘预置运行时与依赖（按平台打包）
  - 首次激活技能时按 hash 安装依赖；命中缓存则复用环境
  - 仅允许受信源安装（默认官方源或学校镜像白名单）
  - 可选开启“仅哈希锁定依赖”模式（更强供应链安全）
- 安全策略：
  - 禁止技能安装系统级包
  - 禁止写入白名单外路径
  - 所有安装与执行写入审计日志（含 skill/version/env_hash）
- 降级策略：
  - 若无可用 Python 运行时：提示一键初始化运行时（不自动静默安装）
  - 初始化失败：技能标记为不可用并给出修复指引

### 7.1.1.1 uv 一键安装/修复流程（设置中心触发）

- 触发条件：
  - 设置页健康检查显示“未安装/版本过低/检测失败”
  - 首次启用第三方/自定义 Python Skill 时检测未通过
- 执行流程：
  1) 用户点击 `一键安装/修复 uv`
  2) 弹出确认框，明确将执行的安装命令与来源域名
  3) 后端按系统执行：
     - macOS：`curl -LsSf https://astral.sh/uv/install.sh | sh`
     - Windows：`powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"`
  4) 安装完成后刷新当前进程 PATH（含 `~/.local/bin`）
  5) 重新执行 `uv --version` 验证并回填状态
- 企业/校园网兼容：
  - 支持代理环境变量（`HTTPS_PROXY` / `HTTP_PROXY`）
  - 支持系统证书链模式（如 `UV_NATIVE_TLS=true`）
  - 网络受限时提示离线安装包流程，不阻塞主应用
- 失败回退：
  - 安装失败记录审计日志并给出可读错误
  - 提供“复制诊断信息”和“离线安装指引”

### 7.1.1.2 内置 Skills 预置运行时打包策略（面向非技术教师）

- 打包目标：教师安装后即可使用内置 Skills，不出现“缺 Python/缺依赖”门槛。
- 打包内容（按平台分别产物）：
  - 预置 uv 可执行文件（Windows/macOS）
  - 内置 Skills 的锁定依赖（wheel/缓存或已解析环境）
  - 预置 ONNX Runtime 动态库与版本清单
  - 预置 PaddleOCR 模型文件（检测/识别/方向分类）
  - 依赖清单与版本指纹（用于审计与升级）
- 启动策略：
  - 启动时优先检测预置运行时路径
  - 若预置运行时损坏，再进入一键修复流程
- 升级策略：
  - 跟随应用版本升级（统一灰度与回滚）
  - 内置依赖不允许在线任意漂移升级，避免课堂环境不稳定
- 安全策略：
  - 仅加载签名通过的内置运行时与依赖包
  - 依赖版本固定，升级需经过发布流程与回归测试

## 7.2 MCP Server（外部感官）

- MVP 接入原则：受控接入、默认关闭、按学校/教师显式启用
- 传输优先级：`stdio > HTTP/SSE`
- 接入流程：注册 → 权限声明 → 首次授权 → 健康检查 → 可观测接入
- 安全策略：命令白名单、路径沙箱、网络域名白名单、按工具二次确认

MCP 兼容方案（MCP 2025-11-25）：
- 生命周期：`initialize` → `notifications/initialized` → operation → shutdown
- 协议能力：支持 `tools/list`、`tools/call`、`notifications/tools/list_changed`
- stdio 规范：stdin/stdout 仅 JSON-RPC，stderr 仅日志
- HTTP 规范：携带 `MCP-Protocol-Version`，会话模式处理 `MCP-Session-Id`
- 人审要求：敏感工具调用必须用户确认（human-in-the-loop）
- MVP 策略：优先本地 stdio MCP，远程 HTTP MCP 默认关闭

## 7.3 Skill Store 生命周期

- 本节列入 Phase-2：
  - 安装：包签名校验 + 权限预览
  - 更新：灰度更新 + 失败回滚
  - 卸载：依赖检查 + 数据清理策略

---

## 8. 文档处理与文件产物规范

- 输入支持：docx/xlsx/pdf/png/jpg/jpeg（MVP）
- 中间产物统一放置：`/workspace/jobs/{job_id}/`
- 导出策略：
  - 默认可编辑格式优先（docx/xlsx）
  - 可选 PDF 导出
  - 导出前必须人工确认

命名规范：`{班级}_{任务类型}_{日期}_{版本}.{ext}`

---

## 9. 安全、隐私与合规

1. **本地优先**：默认不上传原始学生敏感数据。
2. **数据分级**：学生个人信息、成绩、家庭信息均标记敏感级别。
3. **密钥管理**：API Key 入系统密钥链，不明文写入配置文件。
4. **最小权限**：文件读写按目录授权，外部插件按 scope 授权。
5. **可审计**：关键操作审计日志可导出（归档/擦除/导出）。
6. **可擦除**：按班级/届别一键导出并安全擦除。
7. **高危操作确认门**：外发、覆盖写入、批量改写必须二次确认并展示参数。
8. **AI 禁删红线**：AI/Skills/MCP 均不提供删除文件能力（禁止 `delete/remove/rm`）。
9. **路径白名单**：仅允许访问工作区白名单目录，越权访问直接拒绝。
10. **外发前脱敏**：调用云模型或外部服务前默认脱敏（可配置开关）。
11. **安装安全约束**：
   - 一键安装必须用户明确确认，禁止静默拉起安装
   - 安装来源仅允许白名单域名（默认 `astral.sh` 及其官方跳转域）
   - 安装与修复动作写入审计日志（命令、来源、结果、操作者）

---

## 10. UI 与交互实现约束（工程化映射）

直接映射 PRD UX 约束：

- 三栏布局 + 可折叠（左 220/64，右 360）
- 图标+中文文字（禁止纯图标）
- 所有 AI 输出默认草稿态
- 错误提示中文自然语言化
- 进度白盒：步骤 + 三态 + 剩余时间
- 禁止侵入式弹窗，使用底部 Toast

---

## 11. 里程碑计划（12 周）

## 11.1 迭代节奏

### M1（第 1-2 周）：工程底座
- Tauri 工程初始化、IPC 框架、数据库迁移框架
- 设置中心基础页、存储路径与密钥管理
- 统一任务状态机与日志机制
- Agentic Search 基础能力（SQL + 文件遍历）

### M2（第 3-5 周）：教务数据基座（模块 3）
- 班级/学科/学生档案 CRUD
- Excel 导入、标签系统、成绩曲线、观察记录
- 日程课表与文件关联

### M3（第 6-8 周）：家校沟通（模块 4.3）
- 家长沟通文案（记忆注入）
- 期末评语批量生成 + 依据标注
- 班会活动文案生成

### M4（第 9-10 周）：作业与考评（模块 4.2）
- 作业图片批处理/OCR/归并
- 判分与低置信度人工确认
- 错题重组生成 Word/答案页

### M5（第 11-12 周）：扩展与验收（模块 5 收口）
- Skills 管理、MCP 接入、快捷指令
- 全链路压测、隐私合规检查、UAT 验收

## 11.2 团队建议（最小编制）
- 桌面/前端工程师 2
- Rust/后端工程师 2
- AI 应用工程师 1
- QA 1
- 产品/设计 1（共享）

---

## 12. 风险清单与缓释

1. **OCR 质量受拍照条件影响大**
   - 缓释：预处理增强 + 低置信度人工确认 + 引导拍照规范

2. **多模型供应商行为差异导致结果波动**
   - 缓释：Provider 适配层 + 回归基准集 + 关键场景固定模板

3. **插件/MCP 引入安全风险**
   - 缓释：权限分级 + 白名单 + 沙箱 + 审计日志

4. **批量生成结果重复/空泛**
   - 缓释：依据注入约束 + 反重复策略 + 人审环节

5. **性能压力（批量图片/OCR）**
   - 缓释：任务队列 + 并发控制 + 缓存与断点续跑

---

## 13. 验收标准（Definition of Done）

## 13.1 功能验收
- PRD 第 7 章 4 大模块均具备可演示闭环。
- 至少完成 3 个典型教师场景端到端：
  1) 批量作业结构化；2) 批量期末评语；3) 家长沟通文案。

## 13.2 质量验收
- 关键任务成功率 ≥ 95%
- OCR 结构化字段准确率（样本集）≥ 90%
- 批量任务中断后恢复成功率 ≥ 99%
- 关键页面操作反馈 < 200ms（本地交互）

## 13.3 安全验收
- 密钥不明文落盘
- 导出/归档/擦除可追溯
- 所有 AI 输出默认需教师确认

---

## 14. 技术选型备选与取舍

## 14.1 Tauri vs Electron
- 结论：**MVP 选 Tauri**（体积小、资源占用低、安全边界更清晰）
- 备选：若团队缺 Rust 能力，可在首期采用 Electron + Node Worker，后续迁移

## 14.2 记忆检索选型（MVP）
- 结论：MVP 采用 `SQLite FTS + 文件夹遍历 + Markdown 记忆`，不引入向量库。
- 选型理由：
  1) 教师场景精确检索需求更高；
  2) Markdown 记忆可视、可审、可编辑；
  3) 本地轻量、合规与运维成本低。
- 预留：Phase-2 仅在召回效果不足时评估 SQLite-VSS。

## 14.3 Agent 编排框架选型
- 目标能力：多模型接入、function call/tool loop、提示词管理与版本化。
- 候选结论：
  - Rust Rig：Rust 原生，适合在 Tauri 后端统一实现模型调用、工具调用与代理编排。
  - TS 方案（Vercel/LangGraph）：生态成熟，但会引入前后端职责分散与跨语言编排复杂度。
- 最终方案（MVP）：
  - **统一框架：Rust Rig（后端）**
  - **统一职责：Agent 编排 + LLM Provider 接入 + function call/tool loop 全部在后端实现**
  - **前端职责：仅渲染状态与结果，不承载任何模型密钥与编排逻辑**
  - Prompt 管理：模板文件化 + 版本号 + 变量白名单（目录：`packages/prompt-templates/`），由后端 Rig 加载

说明：该方案减少跨层状态同步与安全暴露面，便于审计、权限控制与问题定位。

## 14.4 OCR 选型
- 结论：MVP 用“通用 OCR 引擎 + 云增强可选”
- 原因：保证复杂拍照场景下的可用性与准确率平衡
- 1.0 强制约束：安装包内置 ONNX Runtime 与 PaddleOCR 模型文件，确保离线可用与跨平台一致性。

## 14.4.1 构建与打包策略（发布流程约束）
- **唯一发布通道：GitHub Actions**
  - Windows/macOS 安装包、内置 uv、内置依赖、ONNX Runtime、PaddleOCR 模型均由 CI 统一构建与签名。
  - 产物命名、版本号、校验摘要（SHA256）由流水线统一生成。
- **本地环境用途：仅预览与调试**
  - 本地可运行开发预览（UI 与功能联调），但不作为正式发布产物来源。
  - 禁止本地手工打包并对外分发，避免环境漂移与依赖不一致。
- 发布质量门禁：
  - CI 必须通过：单测、集成测试、模型文件完整性校验、运行时健康检查。
  - 未通过门禁不得进入发布步骤。

## 14.5 1.0 后端 Agent 角色与 Function Call 设计（Rig）

> 约束前提：Agent 编排与 LLM 接入全部在 Rust 后端 Rig 实现；前端仅负责渲染与交互。

### 14.5.1 Agent 角色定义（中文名称 + 说明）

1) **总控编排 Agent（Orchestrator Agent）**
- 中文说明：任务总调度中枢。负责意图解析、子任务拆分、调用子 Agent、推进状态机、触发人工确认闸门。
- 典型场景：从“批量评语”指令拆成“拉取数据→生成草稿→去重→人工确认→导出”。
- 路由定位（MVP 调整）：
  - 默认不介入“页面内已知意图”任务，避免额外一跳带来的延迟。
  - 仅在**全局唤醒框/无明确UI上下文**时承担意图识别与路由。
  - 当垂直 Agent 返回 `fallback_to_orchestrator=true` 时接管。
- 权限边界：
  - 允许：任务编排类工具、风险评估、审批请求、审计日志写入
  - 禁止：直接改写学生主数据、直接执行高危外发

2) **学生档案 Agent（Profile Agent）**
- 中文说明：负责班级、学生、标签、成绩、观察记录等结构化档案数据处理。
- 典型场景：Excel 导入学生名单、成绩回填、按学生拉取历史观察。
- 权限边界：
  - 允许：学生档案/成绩相关数据库读写、FTS 检索
  - 禁止：执行外部消息发送、安装运行时、调用高危系统命令

3) **作业结构化 Agent（Assignment Structuring Agent）**
- 中文说明：负责作业图片预处理、OCR 识别、答案结构化和初步判分。
- 典型场景：30 张作业照片自动识别并输出 Excel 成绩汇总。
- 权限边界：
  - 允许：作业目录读、任务中间文件写、OCR/判分工具调用
  - 禁止：改写学生档案主表、执行删除文件

4) **错题重组 Agent（Practice Recomposition Agent）**
- 中文说明：基于历史错题与知识点模板，生成个性化练习卷与答案页。
- 典型场景：提取某学生近一个月几何错题，生成 5 道同类变式题。
- 权限边界：
  - 允许：错题检索、题目模板调用、文档生成导出
  - 禁止：直接修改成绩、外发家长消息

5) **家校沟通 Agent（Parent Communication Agent）**
- 中文说明：结合学生档案与沟通历史，生成高情商家长沟通文案草稿。
- 典型场景：针对“作业拖延”生成先肯定再建议的微信话术。
- 权限边界：
  - 允许：档案与记忆只读、文案草稿写入
  - 禁止：未经审批直接发送/外发

6) **评语批量 Agent（Comment Batch Agent）**
- 中文说明：批量生成期末评语，控制重复度并附“依据标注”。
- 典型场景：全班 40 人评语批量生成并可逐条采纳。
- 权限边界：
  - 允许：成绩/观察/标签读取、草稿生成、去重与依据组装
  - 禁止：跳过人工确认直接覆盖正式评语

7) **运行时与扩展 Agent（Runtime & Extension Agent）**
- 中文说明：负责 uv/Python 运行时健康检查、一键安装修复、Skills/MCP 注册与健康检查。
- 典型场景：设置页检测 uv 缺失后，用户确认执行一键安装。
- 权限边界：
  - 允许：运行时检测、受控安装、扩展注册检查
  - 禁止：访问学生敏感正文、执行白名单外网络安装

### 14.5.2 Function Call 工具清单（中文名称 + 详细说明）

> 实现原则：**Agent 的文件操作一律通过 Rust 封装服务（FileService/MemoryService/DocService）调用，不允许通过命令行进行文件读写。**

#### 14.5.2.0 工具暴露瘦身策略（防止 LLM 认知过载）

- 物理工具总量可多，但**单次给 LLM 暴露的工具必须严格限域**。
- 暴露规则：
  1) 每个 Agent 每次请求仅挂载 **5-8 个工具**。
  2) 优先暴露“宏观业务工具（macro tools）”，隐藏底层细粒度工具。
  3) 不同 Agent 的工具集互相隔离，禁止“全量 30 工具同屏暴露”。

Agent 工具作用域（示例）：
- 家校沟通 Agent：`db.query_students`、`memory.search_evidence`、`prompt.load_template`、`prompt.render`、`llm.generate_structured`、`fs.write_copy_on_write`
- 作业结构化 Agent：`assignment.process`、`xlsx.export_grading_summary`、`task.save_checkpoint`、`task.resume_from_checkpoint`
- 评语批量 Agent：`memory.search_evidence`、`prompt.*`、`llm.generate_structured`、`task.*`、`approval.request_user_confirmation`

说明：底层细粒度工具（如 `memory.search_sql_fts`、`memory.search_student_files`）保留在 Rust 内部编排层，不直接暴露给 LLM。

防冲突规则（必须）：
- 当 `memory.search_evidence` 对 LLM 暴露时，`memory.search_sql_fts` 与 `memory.search_student_files` 必须从同一请求的 tools 列表中移除。
- 当 `assignment.process` 对 LLM 暴露时，`image.preprocess_batch`、`ocr.extract_answers`、`grading.*` 默认不对 LLM 直接暴露。

#### A. 数据与记忆工具

1) `db.query_students`
- 中文名称：**学生档案查询工具**
- 说明：按班级/姓名/标签/时间条件查询学生基础信息与档案摘要。
- 输入：过滤条件（class_id、keyword、tags、limit）
- 输出：学生列表（id、姓名、学号、标签、摘要）
- 风险级别：低

2) `db.upsert_student`
- 中文名称：**学生档案写入工具**
- 说明：新增或更新学生基础档案（非敏感高危字段）。
- 输入：学生对象（id、name、student_no、class_id 等）
- 输出：写入结果（success、affected_rows）
- 风险级别：中（写操作）

3) `db.insert_score`
- 中文名称：**成绩写入工具**
- 说明：写入单次考试成绩记录，可用于后续统计与趋势分析。
- 输入：student_id、exam_name、subject、score、full_score、exam_date
- 输出：记录ID
- 风险级别：中（写操作）

4) `db.compute_rank`
- 中文名称：**班级排名计算工具**
- 说明：按考试维度计算班级排名并回填统计字段。
- 输入：class_id、exam_name、subject
- 输出：排名结果摘要（参与人数、更新条数）
- 风险级别：中（批量写）

5) `memory.search_sql_fts`
- 中文名称：**记忆全文检索工具（SQLite FTS）**
- 说明：检索观察记录、历史评语、沟通文本等结构化文本证据。
- 输入：keyword、student_id/class_id、time_window、top_k
- 输出：证据片段列表（内容、来源、时间、相关度）
- 风险级别：低
- 实现约束：查询 `memory_fts` 统一索引表（source_table/source_id/content），禁止退化为全表 LIKE 扫描。

5.0) `memory.search_evidence`
- 中文名称：**统一证据检索工具（对 LLM 暴露）**
- 说明：统一完成“SQL FTS + 文件证据补充 + 去重重排”，对 LLM 隐藏底层检索复杂度。
- 输入：keyword、student_id/class_id、time_window、top_k
- 输出：evidence_items（content、source、timestamp、score）
- 风险级别：低
- 实现约束：
  - 先执行 `memory.search_sql_fts`；
  - 证据不足时内部调用 `memory.search_student_files` 补充；
  - 对 LLM 仅返回单一工具结果，避免参数混淆。

6) `memory.search_student_files`
- 中文名称：**学生文件检索工具（文件夹遍历）**
- 说明：在学生/班级目录中检索 Markdown 记忆、作业、试卷等文件证据。
- 输入：base_path、pattern、student_id/class_id、top_k
- 输出：文件命中列表（path、type、snippet）
- 风险级别：中（文件读取）
- 实现约束：仅调用 Rust `FileService::search_in_whitelist`，禁止 shell/命令行遍历。

7) `memory.append_markdown_note`
- 中文名称：**Markdown 记忆追加工具**
- 说明：将新观察/沟通要点以模板化 frontmatter 追加到记忆文件。
- 输入：student_id、memory_type、tags、content
- 输出：file_path、append_result
- 风险级别：中（文件写）
- 实现约束：使用 Rust `MemoryService::append_entry` 做格式校验与原子写入。

7.1) `memory.read_student_memory_timeline`
- 中文名称：**学生记忆时间线读取工具**
- 说明：读取学生长期记忆并按时间排序，供沟通/评语/分析调用。
- 输入：student_id、from_date、to_date、section_filter、limit
- 输出：entries（time、section、tags、content）
- 风险级别：中（文件读取）

7.2) `memory.read_student_memory_by_topic`
- 中文名称：**学生专题记忆读取工具**
- 说明：按主题抽取记忆证据并返回可引用片段。
- 输入：student_id、topic、subject、top_k
- 输出：topic_entries（evidence、source_ref、confidence）
- 风险级别：中（文件读取）

#### B. 文件与文档工具

8) `fs.list_whitelist`
- 中文名称：**白名单目录列举工具**
- 说明：仅在授权工作区内列出文件与目录。
- 输入：path、recursive、file_type_filter
- 输出：路径清单
- 风险级别：低

9) `fs.read_whitelist`
- 中文名称：**白名单文件读取工具**
- 说明：读取授权路径文件，支持大小与行数限制。
- 输入：file_path、offset、limit
- 输出：文本/二进制摘要
- 风险级别：中（敏感读取）
- 实现约束：仅使用 Rust 文件 API，禁止命令行读取工具。

10) `fs.write_copy_on_write`
- 中文名称：**副本写入工具（不可覆盖源文件）**
- 说明：以 copy-on-write 方式生成新文件，禁止直接覆盖原文件。
- 输入：source_path、target_path、content/patch
- 输出：新文件路径
- 风险级别：中
- 实现约束：由 Rust `FileService::write_cow` 原子落盘并生成审计记录。

11) `docx.render_from_template`
- 中文名称：**Word 模板渲染工具**
- 说明：将结构化数据渲染到学校模板并输出 docx。
- 输入：template_id、data_json、output_path
- 输出：导出结果
- 风险级别：中（产物写出）

12) `xlsx.import_students`
- 中文名称：**学生名单导入工具（Excel）**
- 说明：按模板读取 Excel 并转换为学生档案写入任务。
- 输入：file_path、template_version
- 输出：导入报告（成功/失败行）
- 风险级别：中（批量写）

13) `xlsx.export_grading_summary`
- 中文名称：**作业成绩汇总导出工具（Excel）**
- 说明：导出包含学号、姓名、题号、得分、置信度的结构化表格。
- 输入：job_id、output_path
- 输出：导出文件路径
- 风险级别：中

#### C. 作业处理工具

14) `image.preprocess_batch`
- 中文名称：**作业图像批处理工具**
- 说明：对拍照作业进行去噪、矫正、分割等预处理。
- 输入：image_paths、profile
- 输出：预处理后文件列表
- 风险级别：低

15) `ocr.extract_answers`
- 中文名称：**作答内容识别工具（OCR）**
- 说明：识别作业中的文本/公式/题块并结构化输出。
- 输入：image_paths、layout_hint
- 输出：题块与识别文本、置信度
- 风险级别：低
- 实现约束：采用 ONNX Runtime 加载 PaddleOCR 模型，不依赖系统命令行。

16) `grading.mm_grade_with_answer`
- 中文名称：**多模态批卷工具（试卷+标准答案）**
- 说明：对学生试卷与标准答案做题目级对齐和判定，输出题目得分与理由。
- 输入：paper_images、answer_key_images_or_text、rubric
- 输出：question_scores、reasoning_snippets、confidence
- 风险级别：中（外部模型调用）
- 降级策略：多模态模型不可用时返回 `degraded_to_ocr=true`，交由 OCR+规则流程继续。

17) `grading.rule_check`
- 中文名称：**规则判分工具**
- 说明：按答案键与规则进行初判，低置信项标记人工复核。
- 输入：recognized_answers、answer_key、rules
- 输出：得分明细、低置信项
- 风险级别：中

18) `grading.fuse_ocr_mm_results`
- 中文名称：**OCR与多模态结果融合工具**
- 说明：融合 OCR 与多模态批卷结果，给出最终分数与冲突项。
- 输入：ocr_result、mm_result、fusion_policy
- 输出：final_scores、conflict_items、final_confidence
- 风险级别：中

18.1) `assignment.process`
- 中文名称：**作业处理宏工具（对 LLM 暴露）**
- 说明：整合“预处理→OCR→多模态判定→规则判分→结果融合”全链路，支持内部降级。
- 输入：job_id、paper_images、answer_key、policy
- 输出：final_scores、conflicts、degraded_flags
- 风险级别：中
- 实现约束：
  - Rust 内部编排调用 `image.preprocess_batch / ocr.extract_answers / grading.*`；
  - 多模态不可用时自动降级，不把降级决策暴露给 LLM。

#### D. 提示词与模型工具

19) `prompt.load_template`
- 中文名称：**提示词模板加载工具**
- 说明：按场景和版本加载提示词模板。
- 输入：template_name、version
- 输出：模板正文与变量定义
- 风险级别：低

20) `prompt.render`
- 中文名称：**提示词渲染工具**
- 说明：将业务变量注入模板，生成最终调用提示词。
- 输入：template、variables
- 输出：rendered_prompt
- 风险级别：低

21) `llm.generate_structured`
- 中文名称：**结构化生成工具（Rig Provider）**
- 说明：调用模型并按 JSON Schema 输出结构化结果。
- 输入：model、prompt、schema、temperature_profile
- 输出：structured_content、raw_text、usage
- 风险级别：中（外部服务调用）

#### E. 风险与审计工具

22) `risk.evaluate_action`
- 中文名称：**风险评估工具**
- 说明：对即将执行的操作进行风险分级（low/medium/high/critical）。
- 输入：action、target、scope
- 输出：risk_level、reason、requires_approval
- 风险级别：低

23) `approval.request_user_confirmation`
- 中文名称：**用户确认闸门工具**
- 说明：对高危操作发起人工确认，未确认不得执行。
- 输入：action_summary、params_preview、risk_level
- 输出：approved/rejected、operator、timestamp
- 风险级别：高（流程控制关键）
- 实现约束：必须使用 `request_id + tokio::sync::oneshot` 异步挂起映射；由 `resolve_confirmation(request_id, result)` 唤醒。

24) `audit.log_event`
- 中文名称：**审计日志写入工具**
- 说明：记录关键动作、参数摘要、操作者与结果。
- 输入：actor、action、target、result、risk_level
- 输出：audit_id
- 风险级别：低

24.1) `task.save_checkpoint`
- 中文名称：**任务检查点保存工具**
- 说明：将长任务分片进度与中间结果落盘，支持崩溃后恢复。
- 输入：task_id、checkpoint_cursor、completed_items、partial_output_path
- 输出：checkpoint_version
- 风险级别：中

24.2) `task.resume_from_checkpoint`
- 中文名称：**任务断点恢复工具**
- 说明：读取检查点并恢复任务执行，跳过已完成分片。
- 输入：task_id
- 输出：resume_plan（pending_items、resume_from）
- 风险级别：中
- 实现约束：从 `async_task` 读取状态与租约，必须遵守 lease 机制避免重复消费。

#### F. 运行时与扩展工具

25) `runtime.check_uv`
- 中文名称：**uv 环境检测工具**
- 说明：检测 uv 可用性、版本与可执行路径。
- 输入：none
- 输出：status、version、resolved_path
- 风险级别：低

26) `runtime.install_uv_with_consent`
- 中文名称：**uv 一键安装/修复工具（需确认）**
- 说明：在用户明确确认后执行 uv 安装/修复并回检。
- 输入：platform、install_source、consent_token
- 输出：install_result、version_after_install
- 风险级别：高（安装行为）

26.1) `runtime.verify_bundled_runtime`
- 中文名称：**内置运行时校验工具**
- 说明：校验安装包预置 uv 与内置 Skills 依赖是否完整可用。
- 输入：platform、bundle_manifest_version
- 输出：is_valid、missing_items、repair_suggestion
- 风险级别：低

27) `skills.list`
- 中文名称：**技能清单发现工具**
- 说明：扫描并返回可用 Skills（项目级覆盖用户级）。
- 输入：scan_scope
- 输出：skill_catalog
- 风险级别：低

28) `skills.activate`
- 中文名称：**技能激活工具**
- 说明：按需加载 SKILL.md 与资源引用（渐进加载）。
- 输入：skill_name
- 输出：skill_content、resource_refs
- 风险级别：中

29) `mcp.list_tools`
- 中文名称：**MCP 工具发现工具**
- 说明：通过 MCP `tools/list` 拉取远端/本地 MCP 工具清单。
- 输入：server_id
- 输出：tools metadata
- 风险级别：中

30) `mcp.call_tool`
- 中文名称：**MCP 工具调用工具（受控）**
- 说明：调用 MCP `tools/call`，敏感操作必须走人工确认。
- 输入：server_id、tool_name、arguments
- 输出：tool_result、is_error
- 风险级别：高

### 14.5.5 工具落地分层（Rust Function Call vs Skills）

#### A. 必须 Rust 原生实现（内核能力）
- 特征：安全边界核心、数据一致性核心、跨平台稳定性要求高。
- 工具：
  - 数据层：`db.*`（查询/写入/排名）
  - 记忆索引层：`memory.search_sql_fts`、`memory.read_*`
  - 文件安全层：`fs.*`、`memory.append_markdown_note`
  - 风险审计层：`risk.*`、`approval.*`、`audit.*`
  - 运行时层：`runtime.*`
  - 扩展网关层：`skills.list`、`skills.activate`、`mcp.*`

#### B. 适合 Skills 开发（可变教学策略）
- 特征：学校差异大、学科差异大、规则经常变化。
- 能力：
  - 批卷规则策略（不同学科 rubric）
  - 学校模板适配（特定导出版式）
  - 特殊学科处理（如化学式、语文作文点评规则）
- 对应工具接口：
  - 可复用 Rust 核心工具作为底座（文件/数据/审计）
  - Skills 主要承载“策略与提示词层”，避免触碰底层安全边界

#### 14.5.5.1 协议大一统（LLM 视角统一 Tool）

- 目标：无论底层是 Rust 原生函数还是 Python Skill，在 `tools:[...]` 中对 LLM 都表现为同一种函数调用协议。
- 统一要求：
  1) 统一 JSON Schema（name/description/inputSchema/outputSchema）。
  2) 统一返回结构（success/data/error/degraded_to）。
  3) 统一审计字段（tool_name、invoke_id、risk_level、duration_ms）。

Rust 侧统一适配接口（概念定义）：
```rust
trait UnifiedTool {
    fn name(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    async fn invoke(&self, input: serde_json::Value) -> ToolResult;
}

trait PythonSkillTool: UnifiedTool {
    async fn invoke_python_skill(&self, input: serde_json::Value) -> ToolResult;
}
```

说明：
- 当 LLM 调用某个“看起来普通的工具”时，Rust 适配层决定它是本地 Rust 实现还是 Python Skill 实现。
- 若是 Python Skill，后端内部通过 uv 子进程执行并回收 stdout，再按统一 ToolResult 返回；对 LLM 完全透明。

### 14.5.6 UI 上下文直达路由（低延迟优先）

- 路由原则：
  - 在明确页面上下文中，前端直接命中对应垂直 Agent。
  - 非必要不经过总控编排 Agent，减少多跳推理延迟。
- 示例：
  - 批卷页面拖入作业 → 直达作业结构化 Agent（调用 `assignment.process`）。
  - 家校沟通页面生成话术 → 直达家校沟通 Agent（调用 `memory.search_evidence` + 生成链）。
  - 全局唤醒框自由输入 → 走总控编排 Agent 路由。
- 回退机制：
  - 垂直 Agent 无法处理时返回 `fallback_to_orchestrator=true`，再交给总控处理。

### 14.5.3 Agent 权限矩阵（1.0）

| Agent（中文名） | 允许工具类别 | 受限工具类别 | 禁止事项 |
|---|---|---|---|
| 总控编排 Agent | 风险评估、审批、审计、路由 | 不直接批量改写业务数据 | 禁止绕过审批闸门 |
| 学生档案 Agent | 数据读写、FTS 检索 | 批量覆盖写需审批 | 禁止外发消息 |
| 作业结构化 Agent | 图像/OCR/判分、汇总导出 | 导出覆盖写需审批 | 禁止改学生主档案 |
| 错题重组 Agent | 错题检索、文档生成 | 批量生成后采纳需审批 | 禁止改成绩 |
| 家校沟通 Agent | 档案只读、文案草稿 | 外发需审批 | 禁止直接发送 |
| 评语批量 Agent | 数据读取、批量草稿 | 全量采纳/覆盖需审批 | 禁止跳过人工确认 |
| 运行时与扩展 Agent | uv 检测安装、技能/MCP 健康检查 | 安装/启用扩展需审批 | 禁止读取学生敏感正文 |

### 14.5.4 全局安全红线（所有 Agent/工具必须遵守）

1. 禁止删除文件：不暴露 `delete/remove/rm` 类函数。
2. 仅白名单路径可读写：越权路径访问直接拒绝并审计。
3. 高危动作必须人工确认：外发、覆盖写、批量变更、安装、远程 MCP 敏感调用。
4. 所有关键动作必须审计：至少记录操作者、参数摘要、结果、时间、风险级别。
5. 前端不持有模型密钥：密钥只在 Rust 后端安全存储与使用。
6. 文件操作禁用命令行：所有文件读写、检索、写入必须走 Rust 封装 API。

---

## 15. 实施起步清单（第一周落地）

1. 建立 monorepo：`apps/desktop`, `packages/ui`, `packages/core`, `packages/skills`
2. 完成 Tauri IPC 骨架与任务状态机
3. 落地 SQLite schema + migration
4. 完成设置中心最小可用（密钥、路径、身份预设）
5. 打通一个端到端 smoke flow：学生档案检索 → 家长沟通草稿生成 → 教师确认 → 导出 docx

---

## 16. 与 PRD 的对应矩阵（节选）

- PRD 3.x → 本方案第 4.2、5、11
- PRD 4.2 → 本方案第 4.3、6、8
- PRD 4.3 → 本方案第 4.4、6
- PRD 5.x → 本方案第 4.1、7、9
- PRD 6.x（UX）→ 本方案第 10

---

## 17. 附录：建议目录结构（实现态）

```text
pure-worker/
  apps/
    desktop/                 # Tauri App (UI + shell)
  crates/
    core/                    # Rust core services
    workflow/                # 任务编排与状态机
    storage/                 # SQLite/File/Agentic-Search abstractions
    skill-runtime/           # Built-in skills runtime
    mcp-gateway/             # MCP client bridge
  packages/
    ui-components/
    prompt-templates/
  doc/
    prd-v1.0.md
    tech-solution-v1.0.md
```

---

如果需要，我可以下一步直接输出《数据库 ER 图 + API 清单 + 任务状态机时序图》作为开发开工包（可直接给研发分工）。

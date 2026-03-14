# WP-AI-009: 行课记录数据库模型实现计划

## 目标
创建行课记录（Lesson Record）领域对象，关联学生表现、作业、试卷等业务数据。

## 现状
- 只有 `schedule_event` 表达日程事件，不是课次
- 学生成绩、作业等只挂在 student_id 上，无法按课次聚合

## 实现内容

### 1. 数据库 Schema
新建表 `lesson_record`：
```sql
- id: TEXT PRIMARY KEY (UUID)
- class_id: TEXT NOT NULL (外键 -> classroom)
- schedule_event_id: TEXT (可空，关联日程)
- subject: TEXT NOT NULL (学科)
- lesson_date: TEXT NOT NULL (上课日期)
- lesson_index: INTEGER (第几节课)
- topic: TEXT (本课主题)
- teaching_goal: TEXT (教学目标)
- homework_summary: TEXT (作业概述)
- teacher_note: TEXT (教师备注)
- status: TEXT (状态: planned/ongoing/completed/cancelled)
- is_deleted: INTEGER NOT NULL DEFAULT 0
- created_at: TEXT
- updated_at: TEXT
```

### 2. 业务表关联
为以下表添加 `lesson_record_id` 字段（可空）：
- `observation_note`: 课堂观察记录
- `score_record`: 成绩记录（允许关联课次测）
- `grading_job`: 批改任务
- `assignment_asset`: 作业资源
- `parent_communication`: 家校沟通（可选，关联课后沟通）

### 3. Rust 模型
- `LessonRecord`: 行课记录模型
- `LessonRecordService`: CRUD 和查询服务
- IPC 命令: list/create/update/delete lesson_record

### 4. 关联查询
- 查询某课次的学生表现汇总
- 查询某课次的作业完成情况
- 查询学生在连续课次中的变化轨迹

## 文件变更
- `migrations/0012_lesson_record.sql`: 新建表和字段迁移
- `src/models/lesson_record.rs`: 模型定义
- `src/services/lesson_record.rs`: 服务实现
- `src/commands/lesson_record.rs`: IPC 命令
- `src/models/observation_note.rs`: 添加 lesson_record_id
- `src/models/score_record.rs`: 添加 lesson_record_id
- ...其他关联模型

## 验收标准
- [ ] lesson_record 表创建成功
- [ ] 所有业务表关联字段添加完成
- [ ] CRUD IPC 命令可用
- [ ] 关联查询接口可用

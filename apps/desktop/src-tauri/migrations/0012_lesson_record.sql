-- WP-AI-009: Lesson Record Database Model Migration
-- Version: 0012
-- Description: Create lesson_record table and link existing tables to lessons
--
-- 变更内容：
-- 1. 新增 lesson_record 表：课程记录表，关联班级、日程事件、教学主题等信息
-- 2. 为 observation_note 表添加 lesson_record_id 外键
-- 3. 为 score_record 表添加 lesson_record_id 外键
-- 4. 为 grading_job 表添加 lesson_record_id 外键
-- 5. 为 assignment_asset 表添加 lesson_record_id 外键
-- 6. 为 parent_communication 表添加 lesson_record_id 外键
-- 7. 添加相关索引

-- ============================================================
-- Lesson Record Table（课程记录表）
-- 记录每次课程的教学信息、作业、状态等
-- ============================================================
CREATE TABLE IF NOT EXISTS lesson_record (
    id TEXT PRIMARY KEY,                             -- 课程记录ID（UUID）
    class_id TEXT NOT NULL,                          -- 班级ID（外键）
    schedule_event_id TEXT,                          -- 关联日程事件ID（可选）
    subject TEXT NOT NULL,                           -- 学科
    lesson_date TEXT NOT NULL,                       -- 课程日期（ISO 8601）
    lesson_index INTEGER,                            -- 当天第几节课
    topic TEXT,                                      -- 教学主题
    teaching_goal TEXT,                              -- 教学目标
    homework_summary TEXT,                           -- 作业摘要
    teacher_note TEXT,                               -- 教师备注
    status TEXT NOT NULL DEFAULT 'planned',          -- 状态：planned/ongoing/completed/cancelled
    is_deleted INTEGER NOT NULL DEFAULT 0,           -- 软删除标记
    created_at TEXT NOT NULL,                        -- 创建时间（ISO 8601）
    updated_at TEXT NOT NULL,                        -- 更新时间（ISO 8601）
    FOREIGN KEY (class_id) REFERENCES classroom(id),
    FOREIGN KEY (schedule_event_id) REFERENCES schedule_event(id)
);

-- ============================================================
-- 为现有表添加 lesson_record_id 外键（可选关联）
-- ============================================================

-- 观察记录表关联课程
ALTER TABLE observation_note ADD COLUMN lesson_record_id TEXT REFERENCES lesson_record(id);

-- 成绩记录表关联课程
ALTER TABLE score_record ADD COLUMN lesson_record_id TEXT REFERENCES lesson_record(id);

-- 批改任务表关联课程
ALTER TABLE grading_job ADD COLUMN lesson_record_id TEXT REFERENCES lesson_record(id);

-- 作业资产表关联课程
ALTER TABLE assignment_asset ADD COLUMN lesson_record_id TEXT REFERENCES lesson_record(id);

-- 家校沟通表关联课程
ALTER TABLE parent_communication ADD COLUMN lesson_record_id TEXT REFERENCES lesson_record(id);

-- ============================================================
-- 索引
-- ============================================================

-- 按班级和日期查询课程
CREATE INDEX IF NOT EXISTS idx_lesson_record_class_date ON lesson_record(class_id, lesson_date) WHERE is_deleted = 0;

-- 按班级、学科和日期查询课程
CREATE INDEX IF NOT EXISTS idx_lesson_record_class_subject ON lesson_record(class_id, subject, lesson_date) WHERE is_deleted = 0;

-- 按日程事件查询课程
CREATE INDEX IF NOT EXISTS idx_lesson_record_event ON lesson_record(schedule_event_id) WHERE is_deleted = 0;

-- 按状态查询课程
CREATE INDEX IF NOT EXISTS idx_lesson_record_status ON lesson_record(status, lesson_date) WHERE is_deleted = 0;

-- 观察记录按课程查询
CREATE INDEX IF NOT EXISTS idx_obs_note_lesson ON observation_note(lesson_record_id) WHERE is_deleted = 0;

-- 成绩记录按课程查询
CREATE INDEX IF NOT EXISTS idx_score_lesson ON score_record(lesson_record_id) WHERE is_deleted = 0;

-- 批改任务按课程查询
CREATE INDEX IF NOT EXISTS idx_grading_job_lesson ON grading_job(lesson_record_id) WHERE is_deleted = 0;

-- 作业资产按课程查询
CREATE INDEX IF NOT EXISTS idx_assignment_asset_lesson ON assignment_asset(lesson_record_id) WHERE is_deleted = 0;

-- 家校沟通按课程查询
CREATE INDEX IF NOT EXISTS idx_parent_comm_lesson ON parent_communication(lesson_record_id) WHERE is_deleted = 0;

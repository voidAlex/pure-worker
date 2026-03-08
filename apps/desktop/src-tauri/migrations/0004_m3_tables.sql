-- 创建 M3 里程碑业务表：学期评语与活动公告
-- Version: 0004
-- Description: 新增 semester_comment、activity_announcement 及评语 FTS 同步触发器与索引

-- ============================================================
-- 学期评语表（FTS 来源）
-- ============================================================
CREATE TABLE IF NOT EXISTS semester_comment (
    id TEXT PRIMARY KEY,                             -- 评语ID（UUID）
    student_id TEXT NOT NULL,                        -- 学生ID
    task_id TEXT,                                    -- 关联的批量任务ID（async_task.id）
    term TEXT NOT NULL,                              -- 学期标识，如 2025-2026-1
    draft TEXT,                                      -- AI 生成的评语草稿
    adopted_text TEXT,                               -- 教师确认后的正式评语
    status TEXT NOT NULL DEFAULT 'draft',            -- 状态：draft / adopted / rejected
    evidence_json TEXT,                              -- 证据来源 JSON（记录依据计数）
    evidence_count INTEGER NOT NULL DEFAULT 0,       -- 依据条数
    is_deleted INTEGER NOT NULL DEFAULT 0,           -- 软删除标记
    created_at TEXT NOT NULL,                        -- 创建时间（ISO 8601）
    updated_at TEXT NOT NULL,                        -- 更新时间（ISO 8601）
    FOREIGN KEY (student_id) REFERENCES student(id),
    FOREIGN KEY (task_id) REFERENCES async_task(id)
);

-- ============================================================
-- 活动公告表（不进入 FTS）
-- ============================================================
CREATE TABLE IF NOT EXISTS activity_announcement (
    id TEXT PRIMARY KEY,                             -- 公告ID（UUID）
    class_id TEXT NOT NULL,                          -- 班级ID
    title TEXT NOT NULL,                             -- 活动/班会标题
    topic TEXT,                                      -- 主题描述
    audience TEXT NOT NULL DEFAULT 'parent',         -- 受众：parent / student / internal
    draft TEXT,                                      -- AI 生成的文案草稿
    adopted_text TEXT,                               -- 采纳后的正式文案
    template_id TEXT,                                -- 关联的校本模板ID
    status TEXT NOT NULL DEFAULT 'draft',            -- 状态：draft / adopted / rejected
    is_deleted INTEGER NOT NULL DEFAULT 0,           -- 软删除标记
    created_at TEXT NOT NULL,                        -- 创建时间（ISO 8601）
    updated_at TEXT NOT NULL,                        -- 更新时间（ISO 8601）
    FOREIGN KEY (class_id) REFERENCES classroom(id),
    FOREIGN KEY (template_id) REFERENCES template_file(id)
);

-- ============================================================
-- semester_comment 的 FTS 同步触发器
-- ============================================================
CREATE TRIGGER IF NOT EXISTS sc_ai AFTER INSERT ON semester_comment
WHEN new.is_deleted = 0
BEGIN
    INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
    VALUES(
        'semester_comment',
        new.id,
        new.student_id,
        (SELECT class_id FROM student WHERE id = new.student_id),
        COALESCE(new.adopted_text, new.draft),
        new.created_at
    );
END;

CREATE TRIGGER IF NOT EXISTS sc_au AFTER UPDATE ON semester_comment
BEGIN
    DELETE FROM memory_fts WHERE source_table = 'semester_comment' AND source_id = old.id;
    INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
    SELECT
        'semester_comment',
        new.id,
        new.student_id,
        (SELECT class_id FROM student WHERE id = new.student_id),
        COALESCE(new.adopted_text, new.draft),
        new.created_at
    WHERE new.is_deleted = 0;
END;

CREATE TRIGGER IF NOT EXISTS sc_ad AFTER DELETE ON semester_comment
BEGIN
    DELETE FROM memory_fts WHERE source_table = 'semester_comment' AND source_id = old.id;
END;

-- ============================================================
-- 索引：按软删除约束优化常用查询
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_sc_student_term ON semester_comment(student_id, term) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_sc_task ON semester_comment(task_id) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_aa_class ON activity_announcement(class_id, created_at) WHERE is_deleted = 0;

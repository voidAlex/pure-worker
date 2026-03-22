-- 【WP-AI-BIZ-005】学期评语批量任务运行时化 - 批量子执行表
-- 支持子执行聚合模型

CREATE TABLE IF NOT EXISTS batch_sub_execution (
    id TEXT PRIMARY KEY,
    parent_task_id TEXT NOT NULL,
    student_id TEXT NOT NULL,
    student_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending/running/completed/failed
    error_code TEXT,
    error_message TEXT,
    result_comment_id TEXT,
    execution_record_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (parent_task_id) REFERENCES async_task(id),
    FOREIGN KEY (result_comment_id) REFERENCES semester_comment(id)
);

-- 索引：按父任务查询
CREATE INDEX IF NOT EXISTS idx_batch_sub_execution_parent ON batch_sub_execution(parent_task_id);

-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS idx_batch_sub_execution_status ON batch_sub_execution(status);

-- 索引：按学生和父任务联合查询
CREATE INDEX IF NOT EXISTS idx_batch_sub_execution_student_parent ON batch_sub_execution(student_id, parent_task_id);

-- 长任务持久化与崩溃恢复 + 人工确认异步挂起 迁移
-- Version: 0008
-- Description: 
--   1. async_task 表新增 input_snapshot / recovering_since 列（P-001, P-003）
--   2. 新建 task_checkpoint_item 表（P-002, P-004 幂等去重）
--   3. approval_request 表新增 updated_at 列（P-010~P-014 完善审批流）

-- ============================================================
-- 1. async_task 新增列
-- ============================================================

-- P-001: 任务创建即持久化输入快照（JSON 序列化原始输入参数）
ALTER TABLE async_task ADD COLUMN input_snapshot TEXT;

-- P-003: 启动恢复标记时间（recovering 状态进入时间，用于超时检测）
ALTER TABLE async_task ADD COLUMN recovering_since TEXT;

-- ============================================================
-- 2. task_checkpoint_item 表（分片提交 + 幂等恢复）
-- ============================================================
-- P-002: 分片提交中间结果
-- P-004: 幂等恢复（task_id + item_id 去重）

CREATE TABLE IF NOT EXISTS task_checkpoint_item (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'completed',
    result_json TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES async_task(id)
);

-- 唯一索引：同一任务的同一 item 不允许重复（幂等去重核心约束）
CREATE UNIQUE INDEX IF NOT EXISTS idx_checkpoint_item_dedup
    ON task_checkpoint_item(task_id, item_id);

-- 按任务查询所有已完成 item 的索引
CREATE INDEX IF NOT EXISTS idx_checkpoint_item_task
    ON task_checkpoint_item(task_id, status);

-- ============================================================
-- 3. approval_request 新增 updated_at 列
-- ============================================================
-- P-010~P-014: 完善审批请求生命周期管理

ALTER TABLE approval_request ADD COLUMN updated_at TEXT;

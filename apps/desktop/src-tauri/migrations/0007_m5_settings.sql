-- M5 系统设置与个性化迁移
-- Version: 0007
-- Description: 新增设置相关表、扩展 skill_registry/mcp_server_registry 并初始化默认数据
--
-- 说明：
-- 1. 所有新表使用 TEXT 主键（UUID）
-- 2. 所有新表包含软删除字段 is_deleted（默认 0）
-- 3. 所有新表包含 created_at / updated_at（ISO 8601 文本时间）
-- 4. 业务查询需显式包含 WHERE is_deleted = 0

-- ============================================================
-- App Settings Table（应用设置表）
-- 用于存储通用设置、安全设置、导出设置、快捷键和监控目录等键值配置
-- ============================================================
CREATE TABLE IF NOT EXISTS app_settings (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    category TEXT NOT NULL,
    description TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_app_settings_key
ON app_settings(key) WHERE is_deleted = 0;

-- ============================================================
-- AI Param Preset Table（AI 参数预设表）
-- 用于预置严谨/创意/平衡等生成参数，支持默认与激活态切换
-- ============================================================
CREATE TABLE IF NOT EXISTS ai_param_preset (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    temperature REAL NOT NULL,
    top_p REAL,
    max_tokens INTEGER,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_preset_name
ON ai_param_preset(name) WHERE is_deleted = 0;

-- ============================================================
-- Global Shortcut Table（全局快捷键配置表）
-- 维护动作与快捷键组合的映射，支持启用状态与说明信息
-- ============================================================
CREATE TABLE IF NOT EXISTS global_shortcut (
    id TEXT PRIMARY KEY,
    action TEXT NOT NULL,
    key_combination TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    description TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_shortcut_action
ON global_shortcut(action) WHERE is_deleted = 0;

-- ============================================================
-- Watch Folder Table（监控文件夹配置表）
-- 配置本地目录监控规则，按匹配模式触发自动导入或通知动作
-- ============================================================
CREATE TABLE IF NOT EXISTS watch_folder (
    id TEXT PRIMARY KEY,
    folder_path TEXT NOT NULL,
    pattern TEXT,
    action TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_folder_path
ON watch_folder(folder_path) WHERE is_deleted = 0;

-- ============================================================
-- skill_registry 表结构扩展（M5-030 ~ M5-034）
-- 说明：SQLite 仅支持 ADD COLUMN，新增列通过默认值保证迁移兼容性
-- ============================================================
ALTER TABLE skill_registry ADD COLUMN display_name TEXT;
ALTER TABLE skill_registry ADD COLUMN description TEXT;
ALTER TABLE skill_registry ADD COLUMN skill_type TEXT NOT NULL DEFAULT 'builtin';
ALTER TABLE skill_registry ADD COLUMN env_path TEXT;
ALTER TABLE skill_registry ADD COLUMN config_json TEXT;
ALTER TABLE skill_registry ADD COLUMN updated_at TEXT;
ALTER TABLE skill_registry ADD COLUMN health_status TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE skill_registry ADD COLUMN last_health_check TEXT;

-- ============================================================
-- mcp_server_registry 表结构扩展
-- 增加显示信息、健康状态与更新时间等运维字段
-- ============================================================
ALTER TABLE mcp_server_registry ADD COLUMN display_name TEXT;
ALTER TABLE mcp_server_registry ADD COLUMN description TEXT;
ALTER TABLE mcp_server_registry ADD COLUMN health_status TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE mcp_server_registry ADD COLUMN last_health_check TEXT;
ALTER TABLE mcp_server_registry ADD COLUMN updated_at TEXT;

-- ============================================================
-- 默认数据初始化
-- 说明：主键使用固定 UUID，避免运行时生成
-- ============================================================

-- 默认应用设置
INSERT OR IGNORE INTO app_settings (id, key, value, category, description, is_deleted, created_at, updated_at) VALUES
('7f2f1a5b-8dd8-4b95-bfe0-20f2c377dc31', 'workspace_path', '"./workspace"', 'general', '工作区目录路径', 0, datetime('now'), datetime('now')),
('16a5f5f6-90cc-4fc8-a3fc-f22f50a4c6bb', 'language', '"zh-CN"', 'general', '界面语言', 0, datetime('now'), datetime('now')),
('9ff27c47-dde9-43f5-88c1-e17ca982ec5b', 'desensitize_enabled', 'true', 'security', '外发前脱敏开关（默认开启）', 0, datetime('now'), datetime('now')),
('03212466-f967-40ee-8857-be9e66f5e85b', 'high_risk_confirm_enabled', 'true', 'security', '高危操作二次确认（默认开启）', 0, datetime('now'), datetime('now')),
('b4ccf8a7-84ca-46af-b7b6-86c77f1f0e35', 'default_export_format', '"docx"', 'export', '默认导出格式', 0, datetime('now'), datetime('now')),
('8707553f-ff7a-42ef-9f5f-57b2f8b8dd4e', 'default_export_path', '""', 'export', '默认导出路径（空=弹出选择器）', 0, datetime('now'), datetime('now'));

-- 默认 AI 参数预设
INSERT OR IGNORE INTO ai_param_preset (id, name, display_name, temperature, top_p, max_tokens, is_default, is_active, is_deleted, created_at, updated_at) VALUES
('42f0fdbb-3afc-4f8c-bde8-7f01963f9557', 'strict', '严谨模式', 0.3, 0.85, 2048, 1, 0, 0, datetime('now'), datetime('now')),
('5dad95b5-b3c4-4fda-b443-b2a1d09f8cf4', 'creative', '创意模式', 0.9, 0.95, 4096, 1, 0, 0, datetime('now'), datetime('now')),
('2ec4ec42-e07d-4a8a-969a-cbb00ffbe4f6', 'balanced', '平衡模式', 0.7, 0.9, 2048, 1, 1, 0, datetime('now'), datetime('now'));

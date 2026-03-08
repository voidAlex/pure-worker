-- AI 配置表
CREATE TABLE IF NOT EXISTS ai_config (
    id TEXT PRIMARY KEY NOT NULL,
    provider_name TEXT NOT NULL,          -- 'deepseek' / 'qwen' / 'openai' / 'custom'
    display_name TEXT NOT NULL,           -- 显示名称
    base_url TEXT NOT NULL,               -- API 端点
    api_key_encrypted TEXT NOT NULL,      -- 加密后的 API Key（MVP 阶段 Base64 简单编码）
    default_model TEXT NOT NULL,          -- 默认模型名
    is_active INTEGER NOT NULL DEFAULT 0, -- 是否为当前激活的 Provider
    config_json TEXT,                     -- 额外配置（temperature等）
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ai_config_active ON ai_config(is_active) WHERE is_deleted = 0;

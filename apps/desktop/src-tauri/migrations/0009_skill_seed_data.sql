-- 0009_skill_seed_data.sql
-- 描述：插入5个内置技能的种子数据
-- 这些技能为系统默认提供的 Rust 原生实现技能

-- 文档读写技能
INSERT OR IGNORE INTO skill_registry (
    id, name, version, source, permission_scope, status, is_deleted, created_at,
    display_name, description, skill_type, env_path, config_json, updated_at,
    health_status, last_health_check
) VALUES (
    'builtin-skill-office-read-write',
    'office.read_write',
    '1.0.0',
    'builtin',
    'file:read,file:write',
    'enabled',
    0,
    '2026-01-01T00:00:00+00:00',
    '文档读写',
    '读取和写入 Office 文档（Word、Excel），支持内容提取和模板填充。',
    'builtin',
    NULL,
    NULL,
    '2026-01-01T00:00:00+00:00',
    'healthy',
    '2026-01-01T00:00:00+00:00'
);

-- OCR 文字提取技能
INSERT OR IGNORE INTO skill_registry (
    id, name, version, source, permission_scope, status, is_deleted, created_at,
    display_name, description, skill_type, env_path, config_json, updated_at,
    health_status, last_health_check
) VALUES (
    'builtin-skill-ocr-extract',
    'ocr.extract',
    '1.0.0',
    'builtin',
    'file:read',
    'enabled',
    0,
    '2026-01-01T00:00:00+00:00',
    'OCR 文字提取',
    '从图片中提取文字内容，支持作业、试卷等教育场景文档识别。',
    'builtin',
    NULL,
    NULL,
    '2026-01-01T00:00:00+00:00',
    'healthy',
    '2026-01-01T00:00:00+00:00'
);

-- 图像预处理技能
INSERT OR IGNORE INTO skill_registry (
    id, name, version, source, permission_scope, status, is_deleted, created_at,
    display_name, description, skill_type, env_path, config_json, updated_at,
    health_status, last_health_check
) VALUES (
    'builtin-skill-image-preprocess',
    'image.preprocess',
    '1.0.0',
    'builtin',
    'file:read',
    'enabled',
    0,
    '2026-01-01T00:00:00+00:00',
    '图像预处理',
    '图像预处理工具，支持裁剪、旋转、缩放、灰度转换等基础操作。',
    'builtin',
    NULL,
    NULL,
    '2026-01-01T00:00:00+00:00',
    'healthy',
    '2026-01-01T00:00:00+00:00'
);

-- 数学计算技能
INSERT OR IGNORE INTO skill_registry (
    id, name, version, source, permission_scope, status, is_deleted, created_at,
    display_name, description, skill_type, env_path, config_json, updated_at,
    health_status, last_health_check
) VALUES (
    'builtin-skill-math-compute',
    'math.compute',
    '1.0.0',
    'builtin',
    'compute',
    'enabled',
    0,
    '2026-01-01T00:00:00+00:00',
    '数学计算',
    '数学表达式计算引擎，支持四则运算、函数计算和公式求值。',
    'builtin',
    NULL,
    NULL,
    '2026-01-01T00:00:00+00:00',
    'healthy',
    '2026-01-01T00:00:00+00:00'
);

-- 导出渲染技能
INSERT OR IGNORE INTO skill_registry (
    id, name, version, source, permission_scope, status, is_deleted, created_at,
    display_name, description, skill_type, env_path, config_json, updated_at,
    health_status, last_health_check
) VALUES (
    'builtin-skill-export-render',
    'export.render',
    '1.0.0',
    'builtin',
    'file:read,file:write',
    'enabled',
    0,
    '2026-01-01T00:00:00+00:00',
    '导出渲染',
    '将结构化数据渲染为 Word/Excel 文档并导出。',
    'builtin',
    NULL,
    NULL,
    '2026-01-01T00:00:00+00:00',
    'healthy',
    '2026-01-01T00:00:00+00:00'
);

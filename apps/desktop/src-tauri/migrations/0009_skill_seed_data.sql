-- 0009_skill_seed_data.sql
-- 描述：插入5个内置技能的种子数据（含完整 inputSchema/outputSchema）
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
    '{"inputSchema":{"type":"object","required":["operation"],"properties":{"operation":{"type":"string","enum":["read_excel","read_word"],"description":"操作类型"},"file_path":{"type":"string","description":"文件路径"},"sheet_name":{"type":"string","description":"工作表名称（read_excel 可选，默认第一个工作表）"}}},"outputSchema":{"type":"object","properties":{"sheet_name":{"type":"string","description":"工作表名称"},"row_count":{"type":"integer","description":"行数"},"rows":{"type":"array","description":"行数据数组","items":{"type":"array","items":{"type":["string","number","boolean","null"]}}},"paragraphs":{"type":"array","description":"Word 段落文本数组","items":{"type":"string"}}}}}',
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
    '{"inputSchema":{"type":"object","required":["image_path"],"properties":{"image_path":{"type":"string","description":"待识别图片的文件路径（支持 PNG/JPG/BMP/TIFF）"},"language":{"type":"string","description":"识别语言（默认 ch，可选 en）","default":"ch"}}},"outputSchema":{"type":"object","properties":{"text":{"type":"string","description":"识别出的完整文本"},"blocks":{"type":"array","description":"文本块列表（含位置坐标和置信度）","items":{"type":"object","properties":{"text":{"type":"string"},"confidence":{"type":"number"},"bbox":{"type":"array","items":{"type":"number"},"description":"边界框坐标 [x1,y1,x2,y2]"}}}}}}}',
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
    'file:read,file:write',
    'enabled',
    0,
    '2026-01-01T00:00:00+00:00',
    '图像预处理',
    '图像预处理工具，支持裁剪、旋转、缩放、灰度转换等基础操作。',
    'builtin',
    NULL,
    '{"inputSchema":{"type":"object","required":["operation","input_path","output_path"],"properties":{"operation":{"type":"string","enum":["grayscale","resize","rotate"],"description":"操作类型"},"input_path":{"type":"string","description":"输入图片路径"},"output_path":{"type":"string","description":"输出图片路径"},"width":{"type":"integer","description":"目标宽度（resize 操作必填）"},"height":{"type":"integer","description":"目标高度（resize 操作必填）"},"degrees":{"type":"integer","enum":[90,180,270],"description":"旋转角度（rotate 操作必填）"}}},"outputSchema":{"type":"object","properties":{"operation":{"type":"string","description":"执行的操作类型"},"input_path":{"type":"string","description":"输入文件路径"},"output_path":{"type":"string","description":"输出文件路径"},"width":{"type":"integer","description":"处理后图片宽度"},"height":{"type":"integer","description":"处理后图片高度"}}}}',
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
    '{"inputSchema":{"type":"object","required":["expression"],"properties":{"expression":{"type":"string","description":"数学表达式（支持 +、-、*、/、sin、cos、sqrt、abs 等）"}}},"outputSchema":{"type":"object","properties":{"result":{"type":"number","description":"计算结果"},"expression":{"type":"string","description":"原始表达式"}}}}',
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
    '{"inputSchema":{"type":"object","required":["format","output_path"],"properties":{"format":{"type":"string","enum":["docx","xlsx"],"description":"导出格式"},"output_path":{"type":"string","description":"输出文件路径"},"paragraphs":{"type":"array","description":"段落文本数组（docx 格式必填）","items":{"type":"string"}},"rows":{"type":"array","description":"行数据二维数组（xlsx 格式必填）","items":{"type":"array","items":{"type":["string","number","boolean","null"]}}}}},"outputSchema":{"type":"object","properties":{"format":{"type":"string","description":"导出格式"},"output_path":{"type":"string","description":"输出文件路径"},"paragraph_count":{"type":"integer","description":"段落数（docx）"},"row_count":{"type":"integer","description":"行数（xlsx）"},"cell_count":{"type":"integer","description":"单元格数（xlsx）"}}}}',
    '2026-01-01T00:00:00+00:00',
    'healthy',
    '2026-01-01T00:00:00+00:00'
);

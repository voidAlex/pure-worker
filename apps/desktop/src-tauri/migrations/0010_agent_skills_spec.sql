-- 迁移 0010: 添加 Agent Skills 官方规范支持的新字段
-- 创建时间: 2026-03-14
-- 说明: 扩展 skill_registry 表以支持 Agent Skills 官方规范 (https://agentskills.io/specification)

-- 添加 license 字段（许可证名称或引用）
ALTER TABLE skill_registry ADD COLUMN license TEXT;

-- 添加 compatibility 字段（环境兼容性要求）
ALTER TABLE skill_registry ADD COLUMN compatibility TEXT;

-- 添加 metadata_json 字段（元数据，JSON 格式存储）
ALTER TABLE skill_registry ADD COLUMN metadata_json TEXT;

-- 添加 allowed_tools 字段（允许使用的工具列表，空格分隔）
ALTER TABLE skill_registry ADD COLUMN allowed_tools TEXT;

-- 添加 body_content 字段（SKILL.md 正文内容，渐进式加载）
ALTER TABLE skill_registry ADD COLUMN body_content TEXT;

-- 添加 entry_script 字段（入口脚本路径，如 scripts/main.py）
ALTER TABLE skill_registry ADD COLUMN entry_script TEXT;

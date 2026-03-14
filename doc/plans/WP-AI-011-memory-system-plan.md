# WP-AI-011: 教师偏好记忆 / soul.md 机制实现计划

## 目标
建立三层记忆体系，让助手记住教师偏好但保持人可控、可撤销、可审计。

## 现状
- 已有"学生长期记忆"（学生侧）
- 无"教师偏好/助手行为偏好"记忆域
- `app_settings` 只有通用设置，不等于自主记忆

## 实现内容

### 1. 记忆载体

#### A. Session Memory（已实现）
- 当前对话上下文
- 保存在 conversation/conversation_message + summary

#### B. Preference Memory（新增）
文件载体：
- `workspace/soul.md`: 项目级/助手级行为准则
- `workspace/user.md`: 教师个人偏好

结构化存储：
- `teacher_preference_memory` 表:
  - id, preference_key, preference_value, preference_type, source, confirmed_at, is_active

偏好项示例：
- 常用输出风格
- 默认学段/学科语气
- 评语偏好
- 导出格式偏好
- 是否偏好先给结论再给证据

#### C. Long-term Working Memory（新增）
- 近期高频工作任务
- 常用模板
- 常用班级/学生关注点
- 教师反复修订后的表达偏好

### 2. 记忆更新策略
不做"每轮都写记忆"，采用：
1. 用户显式确认记住
2. 高频重复偏好触发候选记忆
3. 重要编辑行为后形成候选记忆卡片
4. 用户确认后写入长期偏好

### 3. 记忆加载机制
- 工作台启动时自动加载 soul.md + user.md
- 生成回复前注入相关偏好上下文
- 支持偏好冲突时的优先级处理

### 4. 记忆管理界面
- 偏好记忆列表查看
- 候选记忆确认/拒绝
- 手动添加/编辑/删除偏好

## 文件变更
- `migrations/0013_teacher_memory.sql`: 教师偏好表
- `src/services/teacher_memory.rs`: 记忆服务
- `src/services/soul_md_manager.rs`: soul.md 管理
- `src/commands/teacher_memory.rs`: IPC 命令
- `src/models/teacher_preference.rs`: 偏好模型

## 验收标准
- [ ] soul.md / user.md 文件读写
- [ ] 偏好记忆结构化存储
- [ ] 候选记忆机制可用
- [ ] 记忆加载注入上下文
- [ ] 偏好管理界面可用

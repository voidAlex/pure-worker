# 贡献指南

## 提交规范

使用 Conventional Commits 格式：

```
type(scope): description
```

### 类型 (type)

- `feat`: 新功能
- `fix`: 错误修复
- `docs`: 文档更新
- `style`: 代码格式（不影响功能）
- `refactor`: 重构
- `test`: 测试
- `chore`: 构建/工具链
- `build`: 构建系统
- `ci`: CI 配置

### 作用域 (scope)

- `desktop`: 桌面应用
- `tauri`: Rust 后端
- `ui`: 前端 UI
- `db`: 数据库
- `ipc`: IPC 通信
- `prompt`: 提示词模板

### 示例

```
feat(ui): 添加学生列表组件
fix(db): 修复 SQLite 连接池泄漏
docs: 更新 API 文档
```

## 开发命令

```bash
# 前端检查
pnpm lint        # ESLint 检查
pnpm format      # Prettier 格式化
pnpm typecheck   # TypeScript 类型检查

# Rust 检查
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 注意事项

- 提交信息使用英文
- 代码注释在必要时使用中文
- 所有 AI 输出必须经过教师确认后才能采纳

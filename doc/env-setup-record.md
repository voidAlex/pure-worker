# 环境初始化记录

> 生成时间：2026-03-07
> 对应开发计划：`doc/development-plan-v1.0.md` 第 1 章

---

## 1. 机器与系统信息

| 项目 | 值 |
|---|---|
| 操作系统 | WSL2 (Ubuntu) on Windows |
| 内核版本 | 6.6.87.2-microsoft-standard-WSL2 |
| 架构 | x86_64 |
| CPU | 8 核 |
| 内存 | 30Gi |
| 用户 | alex |

---

## 2. 工具链版本

| 工具 | 版本 | 要求 | 状态 |
|---|---|---|---|
| Rust (rustc) | 1.87.0 | >= 1.77.2 stable | ✅ |
| Cargo | 1.87.0 | - | ✅ |
| Rustup | 1.28.2 | - | ✅ |
| Node.js | v22.15.0 | >= 20 LTS | ✅ |
| npm | 10.9.2 | - | ✅ |
| pnpm | 10.30.3 | 优先使用 | ✅ |
| Python | 3.12.3 | >= 3.10 | ✅ |
| uv | 0.7.2 | - | ✅ |
| Tauri CLI | 2.10.1 | 2.x | ✅ |

### Rust 已安装工具链

```
stable-x86_64-unknown-linux-gnu (active, default)
1.90.0-x86_64-unknown-linux-gnu

已安装 targets:
  aarch64-apple-darwin
  x86_64-unknown-linux-gnu
```

---

## 3. 国内源配置

### 3.1 Node 生态（npm / pnpm）

```
npm  registry → https://registry.npmmirror.com/
pnpm registry → https://registry.npmmirror.com/
```

配置方式：
- `npm config set registry https://registry.npmmirror.com`
- `pnpm config set registry https://registry.npmmirror.com`
- `~/.zshrc` 中 `export NPM_CONFIG_REGISTRY=https://registry.npmmirror.com/`

### 3.2 Rust 生态（rustup / cargo）

**rustup 镜像**（`~/.zshrc`）：

```bash
export RUSTUP_DIST_SERVER="https://rsproxy.cn"
export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
```

**cargo 镜像**（`~/.cargo/config.toml`）：

```toml
[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[registries.rsproxy]
index = "https://rsproxy.cn/crates.io-index"

[net]
git-fetch-with-cli = true
```

### 3.3 Python / uv

```bash
# PyPI 国内源（~/.zshrc）
export UV_DEFAULT_INDEX="https://mirrors.aliyun.com/pypi/simple"
export UV_INDEX_URL=https://pypi.tuna.tsinghua.edu.cn
export UV_PYPI_INDEX_URL=https://pypi.tuna.tsinghua.edu.cn/simple

# Python 构建包 GitHub 加速（~/.zshrc）
export UV_PYTHON_INSTALL_MIRROR="https://ghproxy.com/https://github.com/astral-sh/python-build-standalone/releases/download"
```

### 3.4 Tauri 下载加速

```bash
# Tauri bundler 工具下载加速（~/.zshrc）
export TAURI_BUNDLER_TOOLS_GITHUB_MIRROR="https://ghproxy.com/https://github.com"
```

---

## 4. 验收清单

- [x] `pnpm`（及备选 `npm`）registry 指向国内源（不要求 yarn）
- [x] `cargo` 通过 rsproxy.cn 加速（`~/.cargo/config.toml` 已配置）
- [x] `rustup` 通过 rsproxy.cn 加速（环境变量已配置）
- [x] `uv` PyPI 源指向阿里云/清华镜像
- [x] `UV_PYTHON_INSTALL_MIRROR` 已配置 GitHub 加速代理
- [x] `TAURI_BUNDLER_TOOLS_GITHUB_MIRROR` 已配置 GitHub 加速代理
- [x] `pnpm tauri info` 成功执行（已于 2026-03-09 在 `apps/desktop` 目录验证）

> 注：`pnpm tauri info` 已完成验证，详见本次执行日志输出。

---

## 5. 配置文件清单

| 文件 | 用途 |
|---|---|
| `~/.zshrc` | Shell 环境变量（Rust/Python/Tauri 镜像） |
| `~/.cargo/config.toml` | Cargo registry 镜像 |
| `~/.npmrc`（或 npm config） | npm registry |
| pnpm 全局配置 | pnpm registry |

---

## 6. 备注

- 当前运行在 WSL2 环境下，Windows 宿主机的 Tauri 构建依赖（WebView2 等）需在 Windows 侧另行确认。
- macOS Homebrew 国内源配置（1.6 节）不适用于当前环境，已跳过。
- `yarn` 未安装，项目统一使用 `pnpm`，无需配置。

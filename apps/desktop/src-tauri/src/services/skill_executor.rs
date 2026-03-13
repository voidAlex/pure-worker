//! 技能执行引擎模块
//!
//! 根据技能类型分发执行：内置技能直接调用 Rust 原生实现，
//! Python 技能通过子进程调用入口脚本。
//! 所有执行均记录审计日志。

use sqlx::SqlitePool;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::skill::SkillRecord;
use crate::services::audit::AuditService;
use crate::services::builtin_skills;
use crate::services::skill::SkillService;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel,
};

/// Python 技能执行超时时间（秒）。
const PYTHON_SKILL_TIMEOUT_SECS: u64 = 60;

/// 技能执行服务。
///
/// 提供统一的技能执行入口，根据技能类型自动分发到对应的执行器。
pub struct SkillExecutorService;

impl SkillExecutorService {
    /// 执行指定技能。
    ///
    /// 根据技能类型分发到内置技能处理器或 Python 子进程执行器。
    /// 执行前校验技能状态和健康状况，执行后记录审计日志。
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `skill_name`: 技能名称
    /// - `input`: JSON 格式的输入参数
    pub async fn execute_skill(
        pool: &SqlitePool,
        skill_name: &str,
        input: serde_json::Value,
    ) -> Result<ToolResult, AppError> {
        let invoke_id = generate_invoke_id();
        let start = Instant::now();

        let (result, skill_id, skill_type, skill_version, skill_env_path) =
            Self::execute_skill_inner(pool, skill_name, &invoke_id, input, &start).await;

        // 审计日志：无论执行结果如何都记录（"finally" 语义）
        let (audit_risk, audit_success, audit_duration) = match &result {
            Ok(r) => (r.audit.risk_level.clone(), r.success, r.audit.duration_ms),
            Err(_) => ("low".to_string(), false, start.elapsed().as_millis() as u64),
        };
        let env_hash = skill_env_path
            .as_deref()
            .map(|p| format!("{:x}", md5_simple(p.as_bytes())));
        let detail = serde_json::json!({
            "invoke_id": invoke_id,
            "skill_name": skill_name,
            "skill_type": skill_type,
            "version": skill_version,
            "env_hash": env_hash,
            "success": audit_success,
            "duration_ms": audit_duration,
        });
        if let Err(e) = AuditService::log_with_detail(
            pool,
            "ai",
            "execute_skill",
            "skill_registry",
            skill_id.as_deref(),
            &audit_risk,
            false,
            Some(&detail.to_string()),
        )
        .await
        {
            eprintln!("[审计日志] 记录技能执行审计失败：{e}");
        }

        result
    }

    /// 技能执行核心逻辑（内部方法）。
    ///
    /// 返回 `(执行结果, 技能ID, 技能类型, 技能版本, 技能环境路径)` 五元组，供外层统一记录审计日志。
    async fn execute_skill_inner(
        pool: &SqlitePool,
        skill_name: &str,
        invoke_id: &str,
        input: serde_json::Value,
        start: &Instant,
    ) -> (
        Result<ToolResult, AppError>,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
    ) {
        let skill = match SkillService::get_skill_by_name(pool, skill_name).await {
            Ok(s) => s,
            Err(e) => {
                return (Err(e), None, "unknown".to_string(), None, None);
            }
        };

        let skill_id = Some(skill.id.clone());
        let skill_type = skill.skill_type.clone();
        let skill_version = skill.version.clone();
        let skill_env_path = skill.env_path.clone();

        if skill.status.as_deref() != Some("enabled") {
            let duration_ms = start.elapsed().as_millis() as u64;
            let result = create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                format!(
                    "技能 '{skill_name}' 未启用，当前状态：{}",
                    skill.status.as_deref().unwrap_or("未知")
                ),
            );
            return (
                Ok(result),
                skill_id,
                skill_type,
                skill_version,
                skill_env_path,
            );
        }

        if skill.health_status == "unhealthy" {
            let duration_ms = start.elapsed().as_millis() as u64;
            let result = create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                format!("技能 '{skill_name}' 健康检查不通过，请先修复"),
            );
            return (
                Ok(result),
                skill_id,
                skill_type,
                skill_version,
                skill_env_path,
            );
        }

        let result = match skill.skill_type.as_str() {
            "builtin" => Self::execute_builtin_skill(skill_name, invoke_id, input, start).await,
            "python" => Self::execute_python_skill(&skill, invoke_id, input, start).await,
            other => {
                let duration_ms = start.elapsed().as_millis() as u64;
                Ok(create_error_result(
                    skill_name,
                    invoke_id,
                    ToolRiskLevel::Low,
                    duration_ms,
                    format!("不支持的技能类型：'{other}'"),
                ))
            }
        };

        (result, skill_id, skill_type, skill_version, skill_env_path)
    }

    /// 执行内置技能。
    ///
    /// 内置技能直接调用 Rust 原生实现，无需子进程。
    /// 通过 `builtin_skills::dispatch_builtin_skill` 分发到对应的技能处理模块。
    async fn execute_builtin_skill(
        skill_name: &str,
        invoke_id: &str,
        input: serde_json::Value,
        start: &Instant,
    ) -> Result<ToolResult, AppError> {
        builtin_skills::dispatch_builtin_skill(skill_name, invoke_id, input, start).await
    }

    /// 执行 Python 技能。
    ///
    /// 通过子进程调用技能 Python 入口脚本，传入 JSON 参数，解析 JSON 输出。
    /// 子进程有超时限制，防止长时间阻塞。
    ///
    /// # 约定
    /// - Python 可执行文件路径：`{env_path}/bin/python`（Unix）或 `{env_path}/Scripts/python.exe`（Windows）
    /// - 入口脚本路径：`{source}/run.py`（source 为技能仓库目录）
    /// - 输入：通过 stdin 传入 JSON 字符串
    /// - 输出：stdout 为 JSON 格式的 ToolResult
    async fn execute_python_skill(
        skill: &SkillRecord,
        invoke_id: &str,
        input: serde_json::Value,
        start: &Instant,
    ) -> Result<ToolResult, AppError> {
        let skill_name = &skill.name;

        // 获取环境路径（venv 目录，用于定位 Python 可执行文件）
        let env_path = skill.env_path.as_deref().ok_or_else(|| {
            AppError::Config(format!("Python 技能 '{skill_name}' 缺少 env_path 配置"))
        })?;

        // 获取技能仓库源目录（用于定位入口脚本 run.py）
        let source_dir = skill.source.as_deref().ok_or_else(|| {
            AppError::Config(format!(
                "Python 技能 '{skill_name}' 缺少 source（仓库目录）配置"
            ))
        })?;

        // 构建 Python 可执行文件路径
        let python_path = if cfg!(windows) {
            std::path::Path::new(env_path)
                .join("Scripts")
                .join("python.exe")
        } else {
            std::path::Path::new(env_path).join("bin").join("python")
        };

        // 构建入口脚本路径（位于技能仓库目录，而非 venv 目录）
        let entry_script = std::path::Path::new(source_dir).join("run.py");

        // 校验文件是否存在
        if !python_path.exists() {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                format!("Python 可执行文件不存在：{}", python_path.display()),
            ));
        }

        if !entry_script.exists() {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                format!("技能入口脚本不存在：{}", entry_script.display()),
            ));
        }

        // 序列化输入参数
        let input_json = serde_json::to_string(&input)
            .map_err(|e| AppError::InvalidInput(format!("序列化技能输入参数失败：{e}")))?;

        // 创建子进程（kill_on_drop 确保超时/异常时子进程被显式终止）
        let mut child = Command::new(&python_path)
            .arg(&entry_script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                AppError::TaskExecution(format!(
                    "启动 Python 子进程失败（技能 '{skill_name}'）：{e}"
                ))
            })?;

        // 写入 stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input_json.as_bytes())
                .await
                .map_err(|e| AppError::TaskExecution(format!("向技能子进程写入数据失败：{e}")))?;
            stdin
                .shutdown()
                .await
                .map_err(|e| AppError::TaskExecution(format!("关闭技能子进程 stdin 失败：{e}")))?;
        }

        // 等待子进程完成（带超时）
        let timeout_duration = std::time::Duration::from_secs(PYTHON_SKILL_TIMEOUT_SECS);
        // wait_with_output() 会消费 child 所有权。
        // 超时时 future 被 drop，tokio::process::Child 的 Drop 实现会自动 kill 子进程。
        let output = match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
            Ok(result) => result
                .map_err(|e| AppError::TaskExecution(format!("等待技能子进程完成失败：{e}")))?,
            Err(_) => {
                // 超时，child 已被 future 持有并将在 drop 时自动终止
                let duration_ms = start.elapsed().as_millis() as u64;
                return Ok(create_error_result(
                    skill_name,
                    invoke_id,
                    ToolRiskLevel::Medium,
                    duration_ms,
                    format!("技能 '{skill_name}' 执行超时（{PYTHON_SKILL_TIMEOUT_SECS} 秒）"),
                ));
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        // 检查退出状态
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                format!(
                    "技能 '{skill_name}' 执行失败（退出码 {}）：{}",
                    output.status, stderr
                ),
            ));
        }

        // 解析 stdout 为 JSON。
        // 尝试解析为完整的 ToolResult 协议格式：
        // - 必须包含 success (bool)、audit (object with tool_name/invoke_id/risk_level/duration_ms)
        // - 校验 audit.tool_name 与预期技能名称一致
        // - 非协议格式 JSON 标记为 degraded_to="raw_json_wrapped"
        let stdout = String::from_utf8_lossy(&output.stdout);
        match serde_json::from_str::<serde_json::Value>(&stdout) {
            Ok(parsed) => {
                // 检查是否符合 ToolResult 协议格式（必须包含 success 布尔字段）
                if let Some(success) = parsed.get("success").and_then(|v| v.as_bool()) {
                    let data = parsed.get("data").cloned();
                    let error = parsed
                        .get("error")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let degraded_to = parsed
                        .get("degraded_to")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    // 解析 audit 子对象，提取 Python 端的审计字段
                    let py_audit = parsed.get("audit");
                    let py_tool_name = py_audit
                        .and_then(|a| a.get("tool_name"))
                        .and_then(|v| v.as_str());
                    let py_invoke_id = py_audit
                        .and_then(|a| a.get("invoke_id"))
                        .and_then(|v| v.as_str());
                    let py_duration_ms = py_audit
                        .and_then(|a| a.get("duration_ms"))
                        .and_then(|v| v.as_u64());

                    // 校验 tool_name 一致性（Python 返回的 tool_name 应与技能名匹配）
                    if let Some(reported_name) = py_tool_name {
                        if reported_name != skill_name {
                            eprintln!(
                                "[技能协议] 技能 '{skill_name}' 返回的 tool_name '{reported_name}' 不一致，以实际技能名为准"
                            );
                        }
                    }

                    // 安全策略：忽略 Python 端自行上报的 risk_level，
                    // 由宿主侧统一确定，防止恶意技能降低风险等级规避审计。
                    let risk_level = ToolRiskLevel::Medium;

                    // invoke_id 一致性校验：始终以 Rust 侧生成的为准，
                    // Python 端返回的仅作日志比对，防止技能脚本篡改调用链标识。
                    if let Some(reported_id) = py_invoke_id {
                        if reported_id != invoke_id {
                            eprintln!(
                                "[技能协议] 技能 '{skill_name}' 返回的 invoke_id '{reported_id}' \
                                 与分配的 '{invoke_id}' 不一致，以分配值为准"
                            );
                        }
                    }
                    let effective_invoke_id = invoke_id;

                    // 采用 Python 端的 duration_ms（如有），否则使用 Rust 侧测量值
                    let effective_duration_ms = py_duration_ms.unwrap_or(duration_ms);

                    let mut result = if success {
                        create_success_result(
                            skill_name,
                            effective_invoke_id,
                            risk_level,
                            effective_duration_ms,
                            data.unwrap_or(serde_json::Value::Null),
                        )
                    } else {
                        create_error_result(
                            skill_name,
                            effective_invoke_id,
                            risk_level,
                            effective_duration_ms,
                            error.unwrap_or_else(|| {
                                "Python 技能返回失败但未提供错误信息".to_string()
                            }),
                        )
                    };
                    result.degraded_to = degraded_to;
                    Ok(result)
                } else {
                    // 非协议格式 JSON，标记为降级包装
                    let mut result = create_success_result(
                        skill_name,
                        invoke_id,
                        ToolRiskLevel::Medium,
                        duration_ms,
                        parsed,
                    );
                    result.degraded_to = Some("raw_json_wrapped".to_string());
                    Ok(result)
                }
            }
            Err(e) => Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                format!(
                    "解析技能 '{skill_name}' 输出失败：{e}，原始输出：{}",
                    stdout.chars().take(500).collect::<String>()
                ),
            )),
        }
    }
}

/// 生成调用唯一标识。
fn generate_invoke_id() -> String {
    Uuid::new_v4().to_string()
}

/// 简易哈希函数（FNV-1a 64 位），用于生成 env_path 的摘要标识。
///
/// 不用于安全场景，仅作审计日志中环境路径的指纹标记。
fn md5_simple(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

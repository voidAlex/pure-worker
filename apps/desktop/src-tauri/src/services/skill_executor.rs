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

        // 查找技能记录
        let skill = SkillService::get_skill_by_name(pool, skill_name).await?;

        // 校验技能状态
        if skill.status.as_deref() != Some("enabled") {
            let duration_ms = start.elapsed().as_millis() as u64;
            let result = create_error_result(
                skill_name,
                &invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                format!(
                    "技能 '{skill_name}' 未启用，当前状态：{}",
                    skill.status.as_deref().unwrap_or("未知")
                ),
            );
            return Ok(result);
        }

        // 校验健康状态
        if skill.health_status == "unhealthy" {
            let duration_ms = start.elapsed().as_millis() as u64;
            let result = create_error_result(
                skill_name,
                &invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                format!("技能 '{skill_name}' 健康检查不通过，请先修复"),
            );
            return Ok(result);
        }

        // 根据技能类型分发执行
        let result = match skill.skill_type.as_str() {
            "builtin" => Self::execute_builtin_skill(skill_name, &invoke_id, input, &start).await,
            "python" => Self::execute_python_skill(&skill, &invoke_id, input, &start).await,
            other => {
                let duration_ms = start.elapsed().as_millis() as u64;
                Ok(create_error_result(
                    skill_name,
                    &invoke_id,
                    ToolRiskLevel::Low,
                    duration_ms,
                    format!("不支持的技能类型：'{other}'"),
                ))
            }
        }?;

        // 记录审计日志
        let detail = serde_json::json!({
            "invoke_id": invoke_id,
            "skill_name": skill_name,
            "skill_type": skill.skill_type,
            "success": result.success,
            "duration_ms": result.audit.duration_ms,
        });
        if let Err(e) = AuditService::log_with_detail(
            pool,
            "ai",
            "execute_skill",
            "skill_registry",
            Some(&skill.id),
            result.audit.risk_level.as_str(),
            false,
            Some(&detail.to_string()),
        )
        .await
        {
            eprintln!("[审计日志] 记录技能执行审计失败：{e}");
        }

        Ok(result)
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
    /// - 入口脚本路径：`{env_path}/run.py`
    /// - 输入：通过 stdin 传入 JSON 字符串
    /// - 输出：stdout 为 JSON 格式的 ToolResult
    async fn execute_python_skill(
        skill: &SkillRecord,
        invoke_id: &str,
        input: serde_json::Value,
        start: &Instant,
    ) -> Result<ToolResult, AppError> {
        let skill_name = &skill.name;

        // 获取环境路径
        let env_path = skill.env_path.as_deref().ok_or_else(|| {
            AppError::Config(format!("Python 技能 '{skill_name}' 缺少 env_path 配置"))
        })?;

        // 构建 Python 可执行文件路径
        let python_path = if cfg!(windows) {
            std::path::Path::new(env_path)
                .join("Scripts")
                .join("python.exe")
        } else {
            std::path::Path::new(env_path).join("bin").join("python")
        };

        // 构建入口脚本路径
        let entry_script = std::path::Path::new(env_path).join("run.py");

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

        // 创建子进程
        let mut child = Command::new(&python_path)
            .arg(&entry_script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
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
        // 优先尝试解析为完整的 ToolResult 协议格式（包含 success/data/error 字段），
        // 若不符合协议格式则将整个 JSON 包装为 data 字段。
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

                    let mut result = if success {
                        create_success_result(
                            skill_name,
                            invoke_id,
                            ToolRiskLevel::Medium,
                            duration_ms,
                            data.unwrap_or(serde_json::Value::Null),
                        )
                    } else {
                        create_error_result(
                            skill_name,
                            invoke_id,
                            ToolRiskLevel::Medium,
                            duration_ms,
                            error.unwrap_or_else(|| {
                                "Python 技能返回失败但未提供错误信息".to_string()
                            }),
                        )
                    };
                    result.degraded_to = degraded_to;
                    Ok(result)
                } else {
                    // 非协议格式，将整个 JSON 包装为 data
                    Ok(create_success_result(
                        skill_name,
                        invoke_id,
                        ToolRiskLevel::Medium,
                        duration_ms,
                        parsed,
                    ))
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

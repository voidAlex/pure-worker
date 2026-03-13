//! 内置技能模块
//!
//! 提供所有 Rust 原生实现的内置技能入口和分发逻辑。
//! 根据技能名称自动路由到对应的技能处理函数。

pub mod export_render;
pub mod image_preprocess;
pub mod math_compute;
pub mod ocr_extract;
pub mod office_read_write;

use std::time::Instant;

use crate::error::AppError;
use crate::services::unified_tool::{create_error_result, ToolResult, ToolRiskLevel};

/// 分发内置技能调用到对应的处理模块。
///
/// 根据技能名称匹配已注册的内置技能，调用其 `execute` 函数。
/// 若技能名称不在已知列表中，返回错误结果。
///
/// # 参数
/// - `skill_name`: 技能名称（如 "math.compute"）
/// - `invoke_id`: 调用唯一标识
/// - `input`: JSON 格式的输入参数
/// - `start`: 计时起点，用于计算执行耗时
pub async fn dispatch_builtin_skill(
    skill_name: &str,
    invoke_id: &str,
    input: serde_json::Value,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    match skill_name {
        "math.compute" => math_compute::execute(input, invoke_id, start).await,
        "image.preprocess" => image_preprocess::execute(input, invoke_id, start).await,
        "ocr.extract" => ocr_extract::execute(input, invoke_id, start).await,
        "office.read_write" => office_read_write::execute(input, invoke_id, start).await,
        "export.render" => export_render::execute(input, invoke_id, start).await,
        _ => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                format!("未知的内置技能：'{skill_name}'"),
            ))
        }
    }
}

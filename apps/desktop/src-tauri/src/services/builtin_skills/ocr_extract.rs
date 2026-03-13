//! OCR 文字提取内置技能模块
//!
//! OCR 引擎（ONNX Runtime + PaddleOCR）尚未集成，
//! 当前版本为占位实现，引导用户使用作业批改流程中的 OCR 功能。

use std::time::Instant;

use crate::error::AppError;
use crate::services::unified_tool::{create_error_result, ToolResult, ToolRiskLevel};

/// 执行 OCR 文字提取技能（占位实现）。
///
/// 当前 ONNX Runtime 尚未集成，此技能返回引导信息，
/// 告知用户使用作业批改流程中已有的 OCR 预处理和识别能力。
///
/// # 输入格式
/// ```json
/// { "image_path": "/path/to/image.png" }
/// ```
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "ocr.extract";

    // 校验输入参数（即使是占位实现也做基本校验）
    let _image_path = match input.get("image_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                "缺少必填参数 'image_path'（待识别图片路径）".to_string(),
            ));
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    Ok(create_error_result(
        skill_name,
        invoke_id,
        ToolRiskLevel::Low,
        duration_ms,
        "OCR 引擎尚未集成，请使用作业批改流程中的 OCR 功能".to_string(),
    ))
}

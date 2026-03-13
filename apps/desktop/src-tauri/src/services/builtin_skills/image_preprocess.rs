//! 图像预处理内置技能模块
//!
//! 提供基础图像处理操作，包括灰度转换、缩放和旋转。
//! 底层使用 image 库，CPU 密集操作通过 spawn_blocking 异步执行。

use std::path::Path;
use std::time::Instant;

use image::ImageReader;

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel,
};

/// 执行图像预处理技能。
///
/// 根据 `operation` 字段分发到对应的图像处理操作。
/// 所有操作均需要 `input_path` 和 `output_path` 参数。
///
/// # 支持的操作
/// - `grayscale`: 灰度转换
/// - `resize`: 缩放（需要 `width` 和 `height` 参数）
/// - `rotate`: 旋转（需要 `degrees` 参数，支持 90/180/270）
///
/// # 输入格式
/// ```json
/// {
///   "operation": "grayscale",
///   "input_path": "/path/to/input.png",
///   "output_path": "/path/to/output.png"
/// }
/// ```
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "image.preprocess";

    // 提取必填参数
    let operation = match input.get("operation").and_then(|v| v.as_str()) {
        Some(op) => op.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                "缺少必填参数 'operation'（操作类型：grayscale/resize/rotate）".to_string(),
            ));
        }
    };

    let input_path = match input.get("input_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                "缺少必填参数 'input_path'（输入图片路径）".to_string(),
            ));
        }
    };

    let output_path = match input.get("output_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                "缺少必填参数 'output_path'（输出图片路径）".to_string(),
            ));
        }
    };

    // 校验输入文件存在
    if !Path::new(&input_path).exists() {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            format!("输入图片文件不存在：{input_path}"),
        ));
    }

    // 克隆参数供 spawn_blocking 使用
    let input_clone = input.clone();
    let op = operation.clone();
    let in_path = input_path.clone();
    let out_path = output_path.clone();

    // 在阻塞线程中执行图像操作
    let result =
        tokio::task::spawn_blocking(move || process_image(&op, &in_path, &out_path, &input_clone))
            .await
            .map_err(|e| AppError::TaskExecution(format!("图像处理任务执行失败：{e}")))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(info) => Ok(create_success_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            info,
        )),
        Err(err_msg) => Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            err_msg,
        )),
    }
}

/// 在阻塞线程中执行具体的图像处理操作。
///
/// 返回 Ok(JSON) 表示成功，Err(String) 表示业务错误。
fn process_image(
    operation: &str,
    input_path: &str,
    output_path: &str,
    input: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let img = ImageReader::open(input_path)
        .map_err(|e| format!("读取图片文件失败：{e}"))?
        .decode()
        .map_err(|e| format!("解码图片失败：{e}"))?;

    let processed = match operation {
        "grayscale" => img.grayscale(),
        "resize" => {
            let width = input
                .get("width")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "缩放操作需要 'width' 参数（正整数）".to_string())?
                as u32;
            let height = input
                .get("height")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "缩放操作需要 'height' 参数（正整数）".to_string())?
                as u32;
            if width == 0 || height == 0 {
                return Err("宽度和高度必须大于 0".to_string());
            }
            img.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        }
        "rotate" => {
            let degrees = input
                .get("degrees")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "旋转操作需要 'degrees' 参数（90/180/270）".to_string())?;
            match degrees {
                90 => img.rotate90(),
                180 => img.rotate180(),
                270 => img.rotate270(),
                _ => return Err(format!("不支持的旋转角度：{degrees}（仅支持 90/180/270）")),
            }
        }
        _ => {
            return Err(format!(
                "不支持的操作类型：'{operation}'（支持：grayscale/resize/rotate）"
            ));
        }
    };

    processed
        .save(output_path)
        .map_err(|e| format!("保存处理后的图片失败：{e}"))?;

    Ok(serde_json::json!({
        "operation": operation,
        "input_path": input_path,
        "output_path": output_path,
        "width": processed.width(),
        "height": processed.height(),
    }))
}

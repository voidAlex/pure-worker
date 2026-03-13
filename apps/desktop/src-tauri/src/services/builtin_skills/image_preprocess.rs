//! 图像预处理内置技能模块
//!
//! 提供基础图像处理操作，包括灰度转换、缩放和旋转。
//! 底层使用 image 库，CPU 密集操作通过 spawn_blocking 异步执行。

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Instant;

use image::ImageReader;

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel, UnifiedTool,
};

/// 图像预处理内置技能。
pub struct ImagePreprocessSkill;

impl UnifiedTool for ImagePreprocessSkill {
    fn name(&self) -> &str {
        "image.preprocess"
    }

    fn description(&self) -> &str {
        "图像预处理：支持灰度转换、缩放、旋转操作"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["grayscale", "resize", "rotate"],
                    "description": "操作类型"
                },
                "input_path": { "type": "string", "description": "输入图片路径" },
                "output_path": { "type": "string", "description": "输出图片路径" },
                "width": { "type": "integer", "description": "缩放目标宽度（resize 时必填）" },
                "height": { "type": "integer", "description": "缩放目标高度（resize 时必填）" },
                "degrees": { "type": "integer", "enum": [90, 180, 270], "description": "旋转角度（rotate 时必填）" }
            },
            "required": ["operation", "input_path", "output_path"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": { "type": "string" },
                "input_path": { "type": "string" },
                "output_path": { "type": "string" },
                "width": { "type": "integer" },
                "height": { "type": "integer" }
            }
        })
    }

    fn risk_level(&self) -> ToolRiskLevel {
        ToolRiskLevel::High
    }

    fn invoke(
        &self,
        input: serde_json::Value,
        invoke_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, AppError>> + Send + '_>> {
        let invoke_id = invoke_id.to_string();
        Box::pin(async move {
            let start = Instant::now();
            execute_inner(input, &invoke_id, &start).await
        })
    }
}

/// 图像预处理内部执行逻辑。
async fn execute_inner(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "image.preprocess";

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

    // 路径白名单校验：输入路径需在读取白名单内，输出路径需在写入白名单内
    if let Err(e) =
        crate::services::path_whitelist::PathWhitelistService::validate_read_path(&input_path)
    {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            format!("输入路径校验失败：{e}"),
        ));
    }
    if let Err(e) =
        crate::services::path_whitelist::PathWhitelistService::validate_write_path(&output_path)
    {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            format!("输出路径校验失败：{e}"),
        ));
    }

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

    let input_clone = input.clone();
    let op = operation.clone();
    let in_path = input_path.clone();
    let out_path = output_path.clone();

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

/// 向后兼容的执行入口。
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    execute_inner(input, invoke_id, start).await
}

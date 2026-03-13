//! 内置技能模块
//!
//! 提供所有 Rust 原生实现的内置技能入口和分发逻辑。
//! 每个技能均实现 UnifiedTool trait，支持 trait 对象分发。

pub mod export_render;
pub mod image_preprocess;
pub mod math_compute;
pub mod ocr_extract;
pub mod office_read_write;

use std::time::Instant;

use crate::error::AppError;
use crate::services::unified_tool::{create_error_result, ToolResult, ToolRiskLevel, UnifiedTool};

/// 获取所有内置技能的 trait 对象列表。
pub fn all_builtin_tools() -> Vec<Box<dyn UnifiedTool>> {
    vec![
        Box::new(math_compute::MathComputeSkill),
        Box::new(image_preprocess::ImagePreprocessSkill),
        Box::new(ocr_extract::OcrExtractSkill),
        Box::new(office_read_write::OfficeReadWriteSkill),
        Box::new(export_render::ExportRenderSkill),
    ]
}

/// 按名称查找内置技能 trait 对象。
pub fn get_builtin_tool(name: &str) -> Option<Box<dyn UnifiedTool>> {
    match name {
        "math.compute" => Some(Box::new(math_compute::MathComputeSkill)),
        "image.preprocess" => Some(Box::new(image_preprocess::ImagePreprocessSkill)),
        "ocr.extract" => Some(Box::new(ocr_extract::OcrExtractSkill)),
        "office.read_write" => Some(Box::new(office_read_write::OfficeReadWriteSkill)),
        "export.render" => Some(Box::new(export_render::ExportRenderSkill)),
        _ => None,
    }
}

/// 分发内置技能调用到对应的处理模块。
///
/// 优先通过 UnifiedTool trait 对象执行。
/// 若技能名称不在已知列表中，返回错误结果。
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

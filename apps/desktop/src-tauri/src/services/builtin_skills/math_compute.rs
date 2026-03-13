//! 数学计算内置技能模块
//!
//! 提供数学表达式求值能力，支持四则运算、数学函数等。
//! 底层使用 meval 库进行安全的表达式解析和计算。

use std::time::Instant;

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel,
};

/// 执行数学计算技能。
///
/// 从输入中提取 `expression` 字段，使用 meval 库解析并求值。
/// 支持基础四则运算和常见数学函数（sin、cos、sqrt、abs 等）。
///
/// # 输入格式
/// ```json
/// { "expression": "2 * (3 + 4)" }
/// ```
///
/// # 输出格式
/// ```json
/// { "result": 14.0, "expression": "2 * (3 + 4)" }
/// ```
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "math.compute";

    // 提取表达式字段
    let expression = match input.get("expression").and_then(|v| v.as_str()) {
        Some(expr) => expr.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                "缺少必填参数 'expression'（数学表达式）".to_string(),
            ));
        }
    };

    // 校验表达式不为空
    if expression.trim().is_empty() {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Low,
            duration_ms,
            "表达式不能为空".to_string(),
        ));
    }

    // 使用 meval 解析并计算表达式
    match meval::eval_str(&expression) {
        Ok(result) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let data = serde_json::json!({
                "result": result,
                "expression": expression,
            });
            Ok(create_success_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                data,
            ))
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                format!("数学表达式计算失败：{e}（表达式：{expression}）"),
            ))
        }
    }
}

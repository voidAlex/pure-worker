//! 数学计算内置技能模块
//!
//! 提供数学表达式求值能力，支持四则运算、数学函数等。
//! 底层使用 meval 库进行安全的表达式解析和计算。

use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel, UnifiedTool,
};

/// 数学计算内置技能。
pub struct MathComputeSkill;

impl UnifiedTool for MathComputeSkill {
    fn name(&self) -> &str {
        "math.compute"
    }

    fn description(&self) -> &str {
        "数学表达式求值，支持四则运算和常见数学函数（sin、cos、sqrt、abs 等）"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "数学表达式，如 '2 * (3 + 4)'"
                }
            },
            "required": ["expression"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "result": { "type": "number", "description": "计算结果" },
                "expression": { "type": "string", "description": "原始表达式" }
            }
        })
    }

    fn risk_level(&self) -> ToolRiskLevel {
        ToolRiskLevel::Low
    }

    fn invoke(
        &self,
        input: serde_json::Value,
        invoke_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, AppError>> + Send + '_>> {
        let invoke_id = invoke_id.to_string();
        Box::pin(async move { execute_inner(input, &invoke_id).await })
    }
}

/// 执行数学计算的内部逻辑。
async fn execute_inner(input: serde_json::Value, invoke_id: &str) -> Result<ToolResult, AppError> {
    let skill_name = "math.compute";
    let start = Instant::now();

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

/// 向后兼容的执行入口（供 dispatch 过渡期使用）。
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "math.compute";

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

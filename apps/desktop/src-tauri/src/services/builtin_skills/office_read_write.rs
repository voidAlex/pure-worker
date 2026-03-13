//! 文档读写内置技能模块
//!
//! 提供 Office 文档（Excel、Word）的读取能力。
//! Excel 读取使用 calamine 库，Word 读取功能开发中。

use std::path::Path;
use std::time::Instant;

use calamine::{open_workbook_auto, Data, Reader};

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel,
};

/// 执行文档读写技能。
///
/// 根据 `operation` 字段分发到对应的文档处理操作。
///
/// # 支持的操作
/// - `read_excel`: 读取 Excel 文件内容，返回 JSON 数组
/// - `read_word`: Word 文档读取（开发中）
///
/// # 输入格式（read_excel）
/// ```json
/// {
///   "operation": "read_excel",
///   "file_path": "/path/to/file.xlsx",
///   "sheet_name": "Sheet1"
/// }
/// ```
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "office.read_write";

    // 提取操作类型
    let operation = match input.get("operation").and_then(|v| v.as_str()) {
        Some(op) => op.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                "缺少必填参数 'operation'（操作类型：read_excel/read_word）".to_string(),
            ));
        }
    };

    match operation.as_str() {
        "read_excel" => execute_read_excel(input, invoke_id, start).await,
        "read_word" => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                "Word 文档读取功能开发中，敬请期待".to_string(),
            ))
        }
        other => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                format!("不支持的操作类型：'{other}'（支持：read_excel/read_word）"),
            ))
        }
    }
}

/// 执行 Excel 文件读取操作。
///
/// 使用 calamine 库打开 Excel 文件，读取指定工作表的内容。
/// 若未指定工作表名称，默认读取第一个工作表。
async fn execute_read_excel(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "office.read_write";

    // 提取文件路径
    let file_path = match input.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                "缺少必填参数 'file_path'（Excel 文件路径）".to_string(),
            ));
        }
    };

    // 校验文件存在
    if !Path::new(&file_path).exists() {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            format!("Excel 文件不存在：{file_path}"),
        ));
    }

    let sheet_name = input
        .get("sheet_name")
        .and_then(|v| v.as_str())
        .map(String::from);

    // 在阻塞线程中读取 Excel
    let result = tokio::task::spawn_blocking(move || read_excel_blocking(&file_path, sheet_name))
        .await
        .map_err(|e| AppError::TaskExecution(format!("Excel 读取任务执行失败：{e}")))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(data) => Ok(create_success_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            data,
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

/// 在阻塞线程中读取 Excel 文件内容。
///
/// 返回 JSON 对象，包含工作表名称和行数据数组。
fn read_excel_blocking(
    file_path: &str,
    sheet_name: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut workbook =
        open_workbook_auto(file_path).map_err(|e| format!("打开 Excel 文件失败：{e}"))?;

    // 确定要读取的工作表名称
    let target_sheet = match sheet_name {
        Some(name) => name,
        None => {
            let sheets = workbook.sheet_names();
            if sheets.is_empty() {
                return Err("Excel 文件中没有工作表".to_string());
            }
            sheets[0].clone()
        }
    };

    // 读取工作表数据
    let range = workbook
        .worksheet_range(&target_sheet)
        .map_err(|e| format!("读取工作表 '{target_sheet}' 失败：{e}"))?;

    let mut rows: Vec<serde_json::Value> = Vec::new();
    for row in range.rows() {
        let cells: Vec<serde_json::Value> = row
            .iter()
            .map(|cell| match cell {
                Data::Empty => serde_json::Value::Null,
                Data::String(s) => serde_json::Value::String(s.clone()),
                Data::Float(f) => serde_json::json!(*f),
                Data::Int(i) => serde_json::json!(*i),
                Data::Bool(b) => serde_json::json!(*b),
                Data::Error(e) => serde_json::Value::String(format!("#ERR:{e:?}")),
                Data::DateTime(dt) => serde_json::Value::String(format!("{dt}")),
                Data::DateTimeIso(s) => serde_json::Value::String(s.clone()),
                Data::DurationIso(s) => serde_json::Value::String(s.clone()),
            })
            .collect();
        rows.push(serde_json::Value::Array(cells));
    }

    Ok(serde_json::json!({
        "sheet_name": target_sheet,
        "row_count": rows.len(),
        "rows": rows,
    }))
}

//! 导出渲染内置技能模块
//!
//! 将结构化数据渲染为 Word（docx）或 Excel（xlsx）文档并导出。
//! 使用 docx-rs 生成 Word 文档，rust_xlsxwriter 生成 Excel 文档。

use std::io::Cursor;
use std::path::Path;
use std::time::Instant;

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel,
};

/// 执行导出渲染技能。
///
/// 根据 `format` 字段选择导出格式，将结构化数据渲染为对应格式的文档。
///
/// # 支持的格式
/// - `docx`: Word 文档（需要 `paragraphs` 和 `output_path` 参数）
/// - `xlsx`: Excel 文档（需要 `rows` 和 `output_path` 参数）
///
/// # 输入格式（docx）
/// ```json
/// {
///   "format": "docx",
///   "output_path": "/path/to/output.docx",
///   "paragraphs": ["第一段", "第二段"]
/// }
/// ```
///
/// # 输入格式（xlsx）
/// ```json
/// {
///   "format": "xlsx",
///   "output_path": "/path/to/output.xlsx",
///   "rows": [["姓名", "成绩"], ["张三", "95"]]
/// }
/// ```
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "export.render";

    // 提取导出格式
    let format = match input.get("format").and_then(|v| v.as_str()) {
        Some(f) => f.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                "缺少必填参数 'format'（导出格式：docx/xlsx）".to_string(),
            ));
        }
    };

    // 提取输出路径
    let output_path = match input.get("output_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                "缺少必填参数 'output_path'（输出文件路径）".to_string(),
            ));
        }
    };

    match format.as_str() {
        "docx" => execute_render_docx(input, invoke_id, start, &output_path).await,
        "xlsx" => execute_render_xlsx(input, invoke_id, start, &output_path).await,
        other => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                format!("不支持的导出格式：'{other}'（支持：docx/xlsx）"),
            ))
        }
    }
}

/// 渲染 Word 文档。
///
/// 从输入中提取段落文本数组，生成 Word 文档并保存到指定路径。
async fn execute_render_docx(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
    output_path: &str,
) -> Result<ToolResult, AppError> {
    let skill_name = "export.render";

    // 提取段落内容
    let paragraphs = match input.get("paragraphs").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<String>>(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                "缺少必填参数 'paragraphs'（段落文本数组）".to_string(),
            ));
        }
    };

    let out = output_path.to_string();

    // 在阻塞线程中生成 Word 文档
    let result = tokio::task::spawn_blocking(move || render_docx_blocking(&paragraphs, &out))
        .await
        .map_err(|e| AppError::TaskExecution(format!("Word 文档渲染任务执行失败：{e}")))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(info) => Ok(create_success_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            info,
        )),
        Err(err_msg) => Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            err_msg,
        )),
    }
}

/// 渲染 Excel 文档。
///
/// 从输入中提取行数据二维数组，生成 Excel 文档并保存到指定路径。
async fn execute_render_xlsx(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
    output_path: &str,
) -> Result<ToolResult, AppError> {
    let skill_name = "export.render";

    // 提取行数据
    let rows = match input.get("rows").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                "缺少必填参数 'rows'（行数据二维数组）".to_string(),
            ));
        }
    };

    let out = output_path.to_string();

    // 在阻塞线程中生成 Excel 文档
    let result = tokio::task::spawn_blocking(move || render_xlsx_blocking(&rows, &out))
        .await
        .map_err(|e| AppError::TaskExecution(format!("Excel 文档渲染任务执行失败：{e}")))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(info) => Ok(create_success_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            info,
        )),
        Err(err_msg) => Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            err_msg,
        )),
    }
}

/// 在阻塞线程中生成 Word 文档。
fn render_docx_blocking(
    paragraphs: &[String],
    output_path: &str,
) -> Result<serde_json::Value, String> {
    use docx_rs::{Docx, Paragraph, Run};

    // 确保输出目录存在
    if let Some(parent) = Path::new(output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败：{e}"))?;
    }

    let mut doc = Docx::new();
    for text in paragraphs {
        let paragraph = Paragraph::new().add_run(Run::new().add_text(text));
        doc = doc.add_paragraph(paragraph);
    }

    let file = std::fs::File::create(output_path).map_err(|e| format!("创建输出文件失败：{e}"))?;

    let mut buf = Cursor::new(Vec::new());
    doc.build()
        .pack(&mut buf)
        .map_err(|e| format!("生成 Word 文档失败：{e}"))?;

    std::io::Write::write_all(&mut std::io::BufWriter::new(file), buf.get_ref())
        .map_err(|e| format!("写入 Word 文档失败：{e}"))?;

    Ok(serde_json::json!({
        "format": "docx",
        "output_path": output_path,
        "paragraph_count": paragraphs.len(),
    }))
}

/// 在阻塞线程中生成 Excel 文档。
fn render_xlsx_blocking(
    rows: &[serde_json::Value],
    output_path: &str,
) -> Result<serde_json::Value, String> {
    use rust_xlsxwriter::Workbook;

    // 确保输出目录存在
    if let Some(parent) = Path::new(output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败：{e}"))?;
    }

    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name("Sheet1")
        .map_err(|e| format!("创建工作表失败：{e}"))?;

    let mut total_cells: usize = 0;

    for (row_idx, row) in rows.iter().enumerate() {
        if let Some(cells) = row.as_array() {
            for (col_idx, cell) in cells.iter().enumerate() {
                let r = row_idx as u32;
                let c = col_idx as u16;
                match cell {
                    serde_json::Value::String(s) => {
                        worksheet
                            .write_string(r, c, s)
                            .map_err(|e| format!("写入单元格失败：{e}"))?;
                    }
                    serde_json::Value::Number(n) => {
                        if let Some(f) = n.as_f64() {
                            worksheet
                                .write_number(r, c, f)
                                .map_err(|e| format!("写入单元格失败：{e}"))?;
                        } else {
                            worksheet
                                .write_string(r, c, n.to_string())
                                .map_err(|e| format!("写入单元格失败：{e}"))?;
                        }
                    }
                    serde_json::Value::Bool(b) => {
                        worksheet
                            .write_boolean(r, c, *b)
                            .map_err(|e| format!("写入单元格失败：{e}"))?;
                    }
                    serde_json::Value::Null => {
                        // 空单元格，跳过
                    }
                    other => {
                        worksheet
                            .write_string(r, c, other.to_string())
                            .map_err(|e| format!("写入单元格失败：{e}"))?;
                    }
                }
                total_cells += 1;
            }
        }
    }

    workbook
        .save(output_path)
        .map_err(|e| format!("保存 Excel 文件失败：{e}"))?;

    Ok(serde_json::json!({
        "format": "xlsx",
        "output_path": output_path,
        "row_count": rows.len(),
        "cell_count": total_cells,
    }))
}

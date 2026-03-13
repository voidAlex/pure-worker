//! 文档读写内置技能模块
//!
//! 提供 Office 文档（Excel、Word）的读取能力。
//! Excel 读取使用 calamine 库，Word 读取使用 zip 解析 docx。

use std::future::Future;
use std::io::Read;
use std::path::Path;
use std::pin::Pin;
use std::time::Instant;

use calamine::{open_workbook_auto, Data, Reader};

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel, UnifiedTool,
};

/// 文档读写内置技能。
pub struct OfficeReadWriteSkill;

impl UnifiedTool for OfficeReadWriteSkill {
    fn name(&self) -> &str {
        "office.read_write"
    }

    fn description(&self) -> &str {
        "Office 文档读取：支持 Excel（xlsx/xls/csv）和 Word（docx）文件读取"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read_excel", "read_word"],
                    "description": "操作类型"
                },
                "file_path": { "type": "string", "description": "文件路径" },
                "sheet_name": { "type": "string", "description": "Excel 工作表名称（可选，默认第一个）" }
            },
            "required": ["operation", "file_path"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "sheet_name": { "type": "string" },
                "row_count": { "type": "integer" },
                "rows": { "type": "array" },
                "text": { "type": "string", "description": "Word 文档提取的文本内容" },
                "paragraph_count": { "type": "integer" },
                "paragraphs": { "type": "array", "items": { "type": "string" }, "description": "Word 文档段落文本数组" }
            }
        })
    }

    fn risk_level(&self) -> ToolRiskLevel {
        ToolRiskLevel::Medium
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

/// 文档读写内部执行逻辑。
async fn execute_inner(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "office.read_write";

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
        "read_word" => execute_read_word(input, invoke_id, start).await,
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
async fn execute_read_excel(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "office.read_write";

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

    // 路径白名单校验必须在任何文件系统访问（包括 exists()）之前执行，
    // 防止白名单外路径的存在性探测。
    if let Err(e) =
        crate::services::path_whitelist::PathWhitelistService::validate_read_path(&file_path)
    {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            format!("文件路径校验失败：{e}"),
        ));
    }

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

/// 执行 Word 文档读取操作。
///
/// 使用 zip 库打开 .docx 文件（本质上是 ZIP 包），
/// 读取 word/document.xml 内容，提取 `<w:t>` 标签中的文本。
async fn execute_read_word(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "office.read_write";

    let file_path = match input.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::Medium,
                duration_ms,
                "缺少必填参数 'file_path'（Word 文件路径）".to_string(),
            ));
        }
    };

    // 路径白名单校验必须在任何文件系统访问之前执行
    if let Err(e) =
        crate::services::path_whitelist::PathWhitelistService::validate_read_path(&file_path)
    {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            format!("文件路径校验失败：{e}"),
        ));
    }

    if !Path::new(&file_path).exists() {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::Medium,
            duration_ms,
            format!("Word 文件不存在：{file_path}"),
        ));
    }

    let result = tokio::task::spawn_blocking(move || read_word_blocking(&file_path))
        .await
        .map_err(|e| AppError::TaskExecution(format!("Word 读取任务执行失败：{e}")))?;

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
fn read_excel_blocking(
    file_path: &str,
    sheet_name: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut workbook =
        open_workbook_auto(file_path).map_err(|e| format!("打开 Excel 文件失败：{e}"))?;

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

/// 在阻塞线程中读取 Word (.docx) 文件内容。
///
/// .docx 文件本质是 ZIP 包，核心文本在 word/document.xml 中。
/// 通过正则提取 `<w:t ...>` 标签内文本，按 `<w:p>` 段落分隔。
fn read_word_blocking(file_path: &str) -> Result<serde_json::Value, String> {
    let file = std::fs::File::open(file_path).map_err(|e| format!("打开 Word 文件失败：{e}"))?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("解析 docx 文件失败（非有效 ZIP）：{e}"))?;

    let mut document_xml = String::new();
    {
        let mut entry = archive
            .by_name("word/document.xml")
            .map_err(|e| format!("docx 中未找到 word/document.xml：{e}"))?;
        entry
            .read_to_string(&mut document_xml)
            .map_err(|e| format!("读取 word/document.xml 失败：{e}"))?;
    }

    // 按 <w:p> 段落拆分，在每个段落内提取 <w:t> 文本
    let paragraphs =
        extract_docx_paragraphs(&document_xml).map_err(|e| format!("解析文档段落失败：{e}"))?;

    let full_text = paragraphs.join("\n");

    Ok(serde_json::json!({
        "text": full_text,
        "paragraph_count": paragraphs.len(),
        "paragraphs": paragraphs,
    }))
}

/// 从 document.xml 内容中提取段落文本。
///
/// 使用正则按 `<w:p>` 拆分段落，再提取每个段落中的 `<w:t>` 文本内容。
/// 同一段落内多个 `<w:t>` 的文本会拼接在一起。
fn extract_docx_paragraphs(xml: &str) -> Result<Vec<String>, String> {
    let para_re = regex::Regex::new(r"(?s)<w:p[ >].*?</w:p>")
        .map_err(|e| format!("段落正则编译失败：{e}"))?;

    let text_re = regex::Regex::new(r"<w:t[^>]*>([^<]*)</w:t>")
        .map_err(|e| format!("w:t 正则编译失败：{e}"))?;

    let mut paragraphs = Vec::new();

    for para_match in para_re.find_iter(xml) {
        let para_xml = para_match.as_str();
        let mut para_text = String::new();

        for cap in text_re.captures_iter(para_xml) {
            if let Some(text) = cap.get(1) {
                para_text.push_str(text.as_str());
            }
        }

        // 只保留非空段落
        if !para_text.is_empty() {
            paragraphs.push(para_text);
        }
    }

    Ok(paragraphs)
}

/// 向后兼容的执行入口。
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    execute_inner(input, invoke_id, start).await
}

//! OCR 文字提取内置技能模块
//!
//! 基于 ONNX Runtime + PaddleOCR v4 模型实现中英文 OCR。
//! 采用检测（det）+ 识别（rec）两阶段流水线：
//! 1. 检测模型定位图片中的文字区域（输出多边形框）
//! 2. 识别模型逐区域提取文字内容
//!
//! 模型文件需放置在 `~/.pureworker/models/` 目录下：
//! - `ch_PP-OCRv4_det_infer.onnx`（检测模型）
//! - `ch_PP-OCRv4_rec_infer.onnx`（识别模型）
//! - `ppocr_keys_v1.txt`（识别字典）

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::OnceLock;
use std::time::Instant;

use image::{DynamicImage, ImageReader, Rgb, RgbImage};
use ort::session::Session;

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel, UnifiedTool,
};

/// OCR 文字提取内置技能。
pub struct OcrExtractSkill;

const SKILL_NAME: &str = "ocr.extract";
const DET_MODEL_FILE: &str = "ch_PP-OCRv4_det_infer.onnx";
const REC_MODEL_FILE: &str = "ch_PP-OCRv4_rec_infer.onnx";
const DICT_FILE: &str = "ppocr_keys_v1.txt";

/// 检测模型的标准输入尺寸（宽高均需为 32 的倍数）
const DET_TARGET_SIZE: u32 = 960;
/// 识别模型的标准输入高度
const REC_IMG_HEIGHT: u32 = 48;
/// 识别模型的标准输入宽度
const REC_IMG_WIDTH: u32 = 320;
/// DB 后处理二值化阈值
const DB_THRESH: f32 = 0.3;
/// DB 后处理 box 过滤阈值
const DB_BOX_THRESH: f32 = 0.6;
/// 最小文字区域边长（像素）
const MIN_SIZE: u32 = 3;

/// PaddleOCR 归一化参数（mean=0.485,0.456,0.406; std=1/0.229,1/0.224,1/0.225）
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD_INV: [f32; 3] = [1.0 / 0.229, 1.0 / 0.224, 1.0 / 0.225];

/// 模型目录全局缓存
static MODELS_DIR: OnceLock<PathBuf> = OnceLock::new();

impl UnifiedTool for OcrExtractSkill {
    fn name(&self) -> &str {
        SKILL_NAME
    }

    fn description(&self) -> &str {
        "OCR 文字识别：从图片中提取文字内容（ONNX Runtime + PaddleOCR v4）"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "image_path": {
                    "type": "string",
                    "description": "待识别图片的文件路径"
                },
                "language": {
                    "type": "string",
                    "enum": ["ch", "en"],
                    "description": "识别语言，默认 ch（中文）",
                    "default": "ch"
                }
            },
            "required": ["image_path"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "识别出的完整文本" },
                "regions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "text": { "type": "string" },
                            "confidence": { "type": "number" },
                            "bbox": { "type": "array", "items": { "type": "number" } }
                        }
                    }
                }
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
        Box::pin(async move {
            let start = Instant::now();
            execute_inner(input, &invoke_id, &start).await
        })
    }
}

/// OCR 执行核心逻辑。
async fn execute_inner(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let image_path = match input.get("image_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                SKILL_NAME,
                invoke_id,
                ToolRiskLevel::Low,
                duration_ms,
                "缺少必填参数 'image_path'（待识别图片路径）".to_string(),
            ));
        }
    };

    if let Err(e) =
        crate::services::path_whitelist::PathWhitelistService::validate_read_path(&image_path)
    {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            SKILL_NAME,
            invoke_id,
            ToolRiskLevel::Low,
            duration_ms,
            format!("图片路径校验失败：{e}"),
        ));
    }

    // 校验模型文件是否就位
    let models_dir = get_models_dir();
    let det_path = models_dir.join(DET_MODEL_FILE);
    let rec_path = models_dir.join(REC_MODEL_FILE);
    let dict_path = models_dir.join(DICT_FILE);

    if !det_path.exists() || !rec_path.exists() || !dict_path.exists() {
        let duration_ms = start.elapsed().as_millis() as u64;
        let missing: Vec<&str> = [
            (!det_path.exists()).then_some(DET_MODEL_FILE),
            (!rec_path.exists()).then_some(REC_MODEL_FILE),
            (!dict_path.exists()).then_some(DICT_FILE),
        ]
        .into_iter()
        .flatten()
        .collect();

        return Ok(create_error_result(
            SKILL_NAME,
            invoke_id,
            ToolRiskLevel::Low,
            duration_ms,
            format!(
                "OCR 模型文件缺失，请将以下文件放置到 {} 目录：{}。\n\
                 可从 PaddlePaddle 官方仓库下载 PP-OCRv4 ONNX 模型。",
                models_dir.display(),
                missing.join("、")
            ),
        ));
    }

    // 在阻塞线程中执行 ONNX 推理（CPU 密集型操作）
    let invoke_id_owned = invoke_id.to_string();

    let result = tokio::task::spawn_blocking(move || {
        run_ocr_pipeline(&image_path, &det_path, &rec_path, &dict_path)
    })
    .await
    .map_err(|e| AppError::TaskExecution(format!("OCR 推理线程异常：{e}")))?;

    match result {
        Ok(regions) => {
            let full_text: String = regions
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            let regions_json: Vec<serde_json::Value> = regions
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "text": r.text,
                        "confidence": r.confidence,
                        "bbox": r.bbox,
                    })
                })
                .collect();

            let elapsed_total = start.elapsed().as_millis() as u64;
            Ok(create_success_result(
                SKILL_NAME,
                &invoke_id_owned,
                ToolRiskLevel::Low,
                elapsed_total,
                serde_json::json!({
                    "text": full_text,
                    "regions": regions_json,
                }),
            ))
        }
        Err(e) => {
            let elapsed_total = start.elapsed().as_millis() as u64;
            Ok(create_error_result(
                SKILL_NAME,
                &invoke_id_owned,
                ToolRiskLevel::Low,
                elapsed_total,
                format!("OCR 识别失败：{e}"),
            ))
        }
    }
}

/// 向后兼容的执行入口。
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    execute_inner(input, invoke_id, start).await
}

// ─── 以下为 ONNX 推理核心实现 ───

/// 单条识别结果
struct OcrRegion {
    text: String,
    confidence: f32,
    bbox: [f32; 4],
}

/// 获取模型文件存放目录（~/.pureworker/models/）
fn get_models_dir() -> &'static PathBuf {
    MODELS_DIR.get_or_init(|| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".pureworker").join("models")
    })
}

/// 完整 OCR 流水线：加载图片 → 检测文字区域 → 逐区域识别
fn run_ocr_pipeline(
    image_path: &str,
    det_model_path: &Path,
    rec_model_path: &Path,
    dict_path: &Path,
) -> Result<Vec<OcrRegion>, String> {
    let img = ImageReader::open(image_path)
        .map_err(|e| format!("打开图片失败：{e}"))?
        .decode()
        .map_err(|e| format!("解码图片失败：{e}"))?;

    let dict = load_dictionary(dict_path)?;

    // 检测阶段
    let mut det_session = Session::builder()
        .map_err(|e| format!("创建检测模型会话构建器失败：{e}"))?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
        .map_err(|e| format!("设置优化级别失败：{e}"))?
        .commit_from_file(det_model_path)
        .map_err(|e| format!("加载检测模型失败：{e}"))?;

    let boxes = run_detection(&mut det_session, &img)?;

    if boxes.is_empty() {
        return Ok(Vec::new());
    }

    // 识别阶段
    let mut rec_session = Session::builder()
        .map_err(|e| format!("创建识别模型会话构建器失败：{e}"))?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
        .map_err(|e| format!("设置优化级别失败：{e}"))?
        .commit_from_file(rec_model_path)
        .map_err(|e| format!("加载识别模型失败：{e}"))?;

    let mut regions = Vec::with_capacity(boxes.len());
    for bbox in &boxes {
        let cropped = crop_text_region(&img, bbox);
        match run_recognition(&mut rec_session, &cropped, &dict) {
            Ok((text, confidence)) => {
                if !text.trim().is_empty() {
                    regions.push(OcrRegion {
                        text,
                        confidence,
                        bbox: *bbox,
                    });
                }
            }
            Err(_) => continue,
        }
    }

    // 按 y 坐标排序（从上到下），y 相同则按 x 排序（从左到右）
    regions.sort_by(|a, b| {
        a.bbox[1]
            .partial_cmp(&b.bbox[1])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.bbox[0]
                    .partial_cmp(&b.bbox[0])
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    Ok(regions)
}

/// 加载 PaddleOCR 字典文件（每行一个字符）
fn load_dictionary(dict_path: &Path) -> Result<Vec<String>, String> {
    let content =
        std::fs::read_to_string(dict_path).map_err(|e| format!("读取字典文件失败：{e}"))?;
    let mut chars: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    // PaddleOCR 字典首位为空白字符（CTC blank）
    chars.insert(0, String::new());
    // 末尾追加空格（space token）
    chars.push(" ".to_string());
    Ok(chars)
}

/// 检测阶段：将图片输入检测模型，通过 DB 后处理提取文字区域的 bounding box。
///
/// PP-OCRv4 det 模型输入：NCHW 格式 [1, 3, H, W]，归一化到 [0,1] 后减均值除标准差。
/// 输出：[1, 1, H, W] 概率图，通过二值化 + 轮廓检测提取文字区域。
fn run_detection(session: &mut Session, img: &DynamicImage) -> Result<Vec<[f32; 4]>, String> {
    let (orig_w, orig_h) = (img.width(), img.height());

    // 将图片缩放到检测模型的标准尺寸（保持长边不超过 DET_TARGET_SIZE，且为 32 的倍数）
    let (det_w, det_h) = compute_det_input_size(orig_w, orig_h);
    let resized = img.resize_exact(det_w, det_h, image::imageops::FilterType::Lanczos3);
    let rgb = resized.to_rgb8();

    // 构建 NCHW 张量 [1, 3, det_h, det_w]
    let mut input_data = vec![0.0_f32; (3 * det_h * det_w) as usize];
    for y in 0..det_h {
        for x in 0..det_w {
            let pixel = rgb.get_pixel(x, y);
            for c in 0..3_usize {
                let val = pixel[c] as f32 / 255.0;
                let normalized = (val - MEAN[c]) * STD_INV[c];
                input_data
                    [c * (det_h * det_w) as usize + y as usize * det_w as usize + x as usize] =
                    normalized;
            }
        }
    }

    let input_tensor = ort::value::Tensor::from_array((
        vec![1_usize, 3, det_h as usize, det_w as usize],
        input_data,
    ))
    .map_err(|e| format!("创建检测输入张量失败：{e}"))?;

    let outputs = session
        .run(ort::inputs![input_tensor])
        .map_err(|e| format!("检测模型推理失败：{e}"))?;

    // 获取输出概率图
    let prob_map: Vec<f32> = {
        let output = outputs.get("sigmoid_0.tmp_0").ok_or("检测模型输出为空")?;
        let (_, prob_slice) = output
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("提取检测概率图失败：{e}"))?;
        prob_slice.to_vec()
    };

    // DB 后处理：二值化 → 查找连通区域 → 提取 bounding box
    let scale_x = orig_w as f32 / det_w as f32;
    let scale_y = orig_h as f32 / det_h as f32;

    let boxes = db_post_process(&prob_map, det_w, det_h, scale_x, scale_y);
    Ok(boxes)
}

/// 计算检测模型输入尺寸（保持长边 ≤ DET_TARGET_SIZE，且宽高为 32 的倍数）
fn compute_det_input_size(orig_w: u32, orig_h: u32) -> (u32, u32) {
    let ratio = if orig_w.max(orig_h) > DET_TARGET_SIZE {
        DET_TARGET_SIZE as f32 / orig_w.max(orig_h) as f32
    } else {
        1.0
    };

    let new_w = ((orig_w as f32 * ratio) as u32).max(32);
    let new_h = ((orig_h as f32 * ratio) as u32).max(32);

    // 对齐到 32 的倍数
    let aligned_w = new_w.div_ceil(32) * 32;
    let aligned_h = new_h.div_ceil(32) * 32;
    (aligned_w, aligned_h)
}

/// DB（Differentiable Binarization）后处理。
///
/// 对概率图进行二值化，通过简单的行扫描查找连通区域，
/// 计算每个区域的最小外接矩形作为文字检测框。
fn db_post_process(
    prob_map: &[f32],
    width: u32,
    height: u32,
    scale_x: f32,
    scale_y: f32,
) -> Vec<[f32; 4]> {
    // 二值化
    let binary: Vec<bool> = prob_map.iter().map(|&p| p > DB_THRESH).collect();

    // 使用简单的连通区域标记（4-连通 flood fill）
    let mut labels = vec![0_u32; (width * height) as usize];
    let mut label_counter = 0_u32;
    let mut boxes = Vec::new();

    let mut grid = BinaryGrid {
        binary: &binary,
        labels: &mut labels,
        width,
        height,
        prob_map,
    };

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            if !grid.binary[idx] || grid.labels[idx] != 0 {
                continue;
            }

            label_counter += 1;
            let (min_x, min_y, max_x, max_y, area, score_sum) =
                flood_fill(&mut grid, x, y, label_counter);

            if area < MIN_SIZE * MIN_SIZE {
                continue;
            }

            let avg_score = score_sum / area as f32;
            if avg_score < DB_BOX_THRESH {
                continue;
            }

            // 映射回原图坐标
            boxes.push([
                min_x as f32 * scale_x,
                min_y as f32 * scale_y,
                max_x as f32 * scale_x,
                max_y as f32 * scale_y,
            ]);
        }
    }

    boxes
}

/// 二值化网格数据（用于 flood fill）
struct BinaryGrid<'a> {
    binary: &'a [bool],
    labels: &'a mut [u32],
    width: u32,
    height: u32,
    prob_map: &'a [f32],
}

/// 4-连通 flood fill，返回 (min_x, min_y, max_x, max_y, area, score_sum)
fn flood_fill(
    grid: &mut BinaryGrid<'_>,
    start_x: u32,
    start_y: u32,
    label: u32,
) -> (u32, u32, u32, u32, u32, f32) {
    let mut stack = vec![(start_x, start_y)];
    let mut min_x = start_x;
    let mut min_y = start_y;
    let mut max_x = start_x;
    let mut max_y = start_y;
    let mut area = 0_u32;
    let mut score_sum = 0.0_f32;

    while let Some((x, y)) = stack.pop() {
        let idx = (y * grid.width + x) as usize;
        if grid.labels[idx] != 0 || !grid.binary[idx] {
            continue;
        }
        grid.labels[idx] = label;
        area += 1;
        score_sum += grid.prob_map[idx];
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);

        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < grid.width {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < grid.height {
            stack.push((x, y + 1));
        }
    }

    (min_x, min_y, max_x, max_y, area, score_sum)
}

/// 从原图中裁切文字区域
fn crop_text_region(img: &DynamicImage, bbox: &[f32; 4]) -> DynamicImage {
    let x = bbox[0].max(0.0) as u32;
    let y = bbox[1].max(0.0) as u32;
    let x2 = (bbox[2] as u32).min(img.width());
    let y2 = (bbox[3] as u32).min(img.height());
    let w = if x2 > x { x2 - x } else { 1 };
    let h = if y2 > y { y2 - y } else { 1 };
    img.crop_imm(x, y, w, h)
}

/// 识别阶段：将裁切的文字区域输入识别模型，CTC 解码输出文字。
///
/// PP-OCRv4 rec 模型输入：NCHW [1, 3, 48, W]（W 按宽高比动态计算，最大 320）。
/// 输出：[1, W/4, dict_size] 的 logits，使用 CTC greedy decode。
fn run_recognition(
    session: &mut Session,
    img: &DynamicImage,
    dict: &[String],
) -> Result<(String, f32), String> {
    let (orig_w, orig_h) = (img.width().max(1), img.height().max(1));

    // 按高度缩放到 REC_IMG_HEIGHT，宽度按比例计算，上限 REC_IMG_WIDTH
    let ratio = REC_IMG_HEIGHT as f32 / orig_h as f32;
    let new_w = ((orig_w as f32 * ratio) as u32).clamp(1, REC_IMG_WIDTH);
    let resized = img.resize_exact(new_w, REC_IMG_HEIGHT, image::imageops::FilterType::Lanczos3);

    // 如果宽度不足 REC_IMG_WIDTH，右侧 padding 灰色
    let mut padded = RgbImage::from_pixel(REC_IMG_WIDTH, REC_IMG_HEIGHT, Rgb([128, 128, 128]));
    let rgb = resized.to_rgb8();
    for y in 0..REC_IMG_HEIGHT {
        for x in 0..new_w.min(REC_IMG_WIDTH) {
            padded.put_pixel(x, y, *rgb.get_pixel(x, y));
        }
    }

    // 构建 NCHW 张量 [1, 3, REC_IMG_HEIGHT, REC_IMG_WIDTH]
    let h = REC_IMG_HEIGHT as usize;
    let w = REC_IMG_WIDTH as usize;
    let mut input_data = vec![0.0_f32; 3 * h * w];
    for py in 0..h {
        for px in 0..w {
            let pixel = padded.get_pixel(px as u32, py as u32);
            for c in 0..3_usize {
                let val = pixel[c] as f32 / 255.0;
                let normalized = (val - MEAN[c]) * STD_INV[c];
                input_data[c * h * w + py * w + px] = normalized;
            }
        }
    }

    let input_tensor = ort::value::Tensor::from_array((vec![1_usize, 3, h, w], input_data))
        .map_err(|e| format!("创建识别输入张量失败：{e}"))?;

    let outputs = session
        .run(ort::inputs![input_tensor])
        .map_err(|e| format!("识别模型推理失败：{e}"))?;

    let output = outputs.values().next().ok_or("识别模型输出为空")?;

    let (logits_shape, logits) = output
        .try_extract_tensor::<f32>()
        .map_err(|e| format!("提取识别 logits 失败：{e}"))?;

    // CTC greedy decode
    let seq_len = logits_shape[1] as usize;
    let dict_size = logits_shape[2] as usize;

    let (text, confidence) = ctc_greedy_decode(logits, seq_len, dict_size, dict);
    Ok((text, confidence))
}

/// CTC greedy decode：对每个时间步取 argmax，去重并移除 blank
fn ctc_greedy_decode(
    logits: &[f32],
    seq_len: usize,
    dict_size: usize,
    dict: &[String],
) -> (String, f32) {
    let mut text = String::new();
    let mut total_confidence = 0.0_f32;
    let mut char_count = 0_u32;
    let mut prev_idx = 0_usize;

    for t in 0..seq_len {
        let offset = t * dict_size;
        let slice = &logits[offset..offset + dict_size];

        // softmax 取最大值
        let max_val = slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_sum: f32 = slice.iter().map(|&v| (v - max_val).exp()).sum();
        let (best_idx, _) = slice
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((0, &0.0));

        let prob = ((slice[best_idx] - max_val).exp()) / exp_sum;

        // 跳过 blank（index 0）和重复字符
        if best_idx != 0 && best_idx != prev_idx {
            if let Some(ch) = dict.get(best_idx) {
                text.push_str(ch);
                total_confidence += prob;
                char_count += 1;
            }
        }

        prev_idx = best_idx;
    }

    let avg_confidence = if char_count > 0 {
        total_confidence / char_count as f32
    } else {
        0.0
    };

    (text, avg_confidence)
}

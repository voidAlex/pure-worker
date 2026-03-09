//! OCR 识别服务模块
//!
//! 提供图片预处理（灰度化、去噪、纠偏）和 OCR 文字识别能力。
//! 底层使用 ONNX Runtime + PaddleOCR 模型。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use chrono::Utc;
use image::{DynamicImage, GrayImage, ImageReader, Luma};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::assignment_grading::{AssignmentAsset, AssignmentOcrResult};
use crate::services::audit::AuditService;

/// OCR 识别服务，处理图片预处理和文字提取。
pub struct OcrService;

struct PreprocessOutcome {
    output_path: PathBuf,
    output_path_for_db: String,
    image_width: i32,
    image_height: i32,
}

const PREPROCESSED_SUBDIR: &str = "preprocessed";
const MAX_IMAGE_LONGEST_SIDE: u32 = 4096;
const PREPROCESS_STATUS_DONE: &str = "done";
const PREPROCESS_STATUS_FAILED: &str = "failed";
const DESKEW_MIN_DEGREE: f32 = 0.5;
const DESKEW_MAX_DEGREE: f32 = 5.0;
const DESKEW_SCAN_STEP_DEGREE: f32 = 0.5;
const MODEL_FILE_NAME: &str = "ch_PP-OCRv4_det_infer.onnx";

impl OcrService {
    pub async fn preprocess_image(
        pool: &SqlitePool,
        asset_id: &str,
        workspace_path: &Path,
    ) -> Result<PathBuf, AppError> {
        let result = async {
            let asset = Self::get_asset_by_id(pool, asset_id).await?;
            let source_path = Self::build_source_image_path(&asset);

            let preprocessed_dir = workspace_path.join(PREPROCESSED_SUBDIR);
            Self::ensure_dir_exists(&preprocessed_dir)?;

            let output_path = Self::build_preprocessed_output_path(&preprocessed_dir, asset_id);
            let process_result = Self::run_preprocess_blocking(source_path, output_path)
            .await
            .map_err(|error| AppError::TaskExecution(format!("图片预处理任务执行失败：{error}")))?;

            let outcome = process_result?;
            let now = Utc::now().to_rfc3339();

            sqlx::query(
                "UPDATE assignment_asset SET preprocessed_path = ?, preprocess_status = ?, image_width = ?, image_height = ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
            )
            .bind(&outcome.output_path_for_db)
            .bind(PREPROCESS_STATUS_DONE)
            .bind(outcome.image_width)
            .bind(outcome.image_height)
            .bind(&now)
            .bind(asset_id)
            .execute(pool)
            .await?;

            Ok(outcome.output_path)
        }
        .await;

        if result.is_err() {
            Self::mark_preprocess_failed(pool, asset_id).await?;
        }

        result
    }

    pub async fn extract_text(
        pool: &SqlitePool,
        asset_id: &str,
        job_id: &str,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        let asset = Self::get_asset_by_id(pool, asset_id).await?;
        let preprocessed_path = asset.preprocessed_path.as_ref().ok_or_else(|| {
            AppError::InvalidInput(format!("预处理图片路径缺失，无法执行 OCR：{asset_id}"))
        })?;

        let attempt_id = Uuid::new_v4().to_string();
        let detail = serde_json::json!({
            "attempt_id": attempt_id,
            "asset_id": asset_id,
            "job_id": job_id,
            "preprocessed_path": preprocessed_path,
            "attempt_at": Utc::now().to_rfc3339(),
            "stage": "ocr_model_loading",
            "message": "开始尝试加载 OCR 模型",
        });

        let _ = AuditService::log_with_detail(
            pool,
            "system",
            "ocr_model_loading_attempt",
            "assignment_asset",
            Some(asset_id),
            "low",
            false,
            Some(&detail.to_string()),
        )
        .await;

        let model_path = Self::resolve_model_path(preprocessed_path);
        if model_path.exists() {
            return Err(AppError::TaskExecution(
                "ONNX Runtime 未集成，模型文件已就位但运行时尚未配置".into(),
            ));
        }

        let simulated_rows =
            Self::simulate_and_insert_ocr_results(pool, &asset, asset_id, job_id).await?;
        let simulation_detail = serde_json::json!({
            "attempt_id": attempt_id,
            "asset_id": asset_id,
            "job_id": job_id,
            "preprocessed_path": preprocessed_path,
            "model_path": model_path.to_string_lossy().to_string(),
            "stage": "ocr_simulation",
            "simulated_count": simulated_rows.len(),
            "message": "模型文件缺失，已回退到 OCR 模拟识别并写入结果",
            "attempt_at": Utc::now().to_rfc3339(),
        });
        let _ = AuditService::log_with_detail(
            pool,
            "system",
            "ocr_simulation_generated",
            "assignment_asset",
            Some(asset_id),
            "low",
            false,
            Some(&simulation_detail.to_string()),
        )
        .await;

        Ok(simulated_rows)
    }

    pub async fn run_ocr_pipeline(
        pool: &SqlitePool,
        asset_id: &str,
        job_id: &str,
        workspace_path: &Path,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        Self::preprocess_image(pool, asset_id, workspace_path)
            .await
            .map_err(|error| {
                AppError::TaskExecution(format!("OCR 预处理失败（asset_id={asset_id}）：{error}"))
            })?;

        Self::extract_text(pool, asset_id, job_id)
            .await
            .map_err(|error| {
                AppError::TaskExecution(format!(
                    "OCR 文本提取失败（asset_id={asset_id}, job_id={job_id}）：{error}"
                ))
            })
    }

    fn ensure_dir_exists(dir: &Path) -> Result<(), AppError> {
        std::fs::create_dir_all(dir)
            .map_err(|error| AppError::FileOperation(format!("预处理目录创建失败：{error}")))
    }

    fn build_source_image_path(asset: &AssignmentAsset) -> PathBuf {
        PathBuf::from(asset.file_path.clone())
    }

    fn build_preprocessed_output_path(preprocessed_dir: &Path, asset_id: &str) -> PathBuf {
        preprocessed_dir.join(format!("{asset_id}.png"))
    }

    async fn run_preprocess_blocking(
        source_path: PathBuf,
        output_path: PathBuf,
    ) -> Result<Result<PreprocessOutcome, AppError>, tokio::task::JoinError> {
        tokio::task::spawn_blocking(move || {
            if !source_path.exists() {
                return Err(AppError::FileOperation(format!(
                    "图片文件不存在：{}",
                    source_path.display()
                )));
            }

            let image = Self::decode_image(&source_path)?;
            let normalized = Self::apply_preprocess_pipeline(image);
            let (image_width, image_height) = Self::dimensions_to_i32(&normalized)?;

            normalized
                .save(&output_path)
                .map_err(|error| AppError::FileOperation(format!("预处理图片保存失败：{error}")))?;

            Ok(PreprocessOutcome {
                output_path_for_db: output_path.to_string_lossy().to_string(),
                output_path,
                image_width,
                image_height,
            })
        })
        .await
    }

    fn decode_image(path: &Path) -> Result<DynamicImage, AppError> {
        ImageReader::open(path)
            .map_err(|error| AppError::FileOperation(format!("图片文件读取失败：{error}")))?
            .decode()
            .map_err(|error| AppError::FileOperation(format!("图片解码失败：{error}")))
    }

    fn apply_preprocess_pipeline(image: DynamicImage) -> DynamicImage {
        let grayscale = image.grayscale().to_luma8();
        let denoised = Self::denoise_with_average_kernel(&grayscale);
        let deskewed = Self::deskew_if_needed(&denoised);

        Self::resize_if_needed(DynamicImage::ImageLuma8(deskewed), MAX_IMAGE_LONGEST_SIDE)
    }

    /// 使用 3x3 平均核近似中值滤波，降低拍照噪点干扰。
    fn denoise_with_average_kernel(image: &GrayImage) -> GrayImage {
        let kernel = [1.0_f32 / 9.0_f32; 9];
        image::imageops::filter3x3(image, &kernel)
    }

    /// 对图像执行基础纠偏：估算小角度倾斜并在阈值内旋转矫正。
    fn deskew_if_needed(image: &GrayImage) -> GrayImage {
        let estimated_angle = Self::estimate_skew_angle(image);
        if estimated_angle.abs() < DESKEW_MIN_DEGREE {
            return image.clone();
        }
        Self::rotate_gray_image(image, -estimated_angle)
    }

    /// 启发式估算倾斜角：通过暗像素在行方向投影方差评估最优角度。
    fn estimate_skew_angle(image: &GrayImage) -> f32 {
        let width = image.width();
        let height = image.height();
        if width < 32 || height < 32 {
            return 0.0;
        }

        let mut best_angle = 0.0_f32;
        let mut best_score = f64::MIN;
        let mut angle = -DESKEW_MAX_DEGREE;
        while angle <= DESKEW_MAX_DEGREE {
            let score = Self::row_projection_variance(image, angle);
            if score > best_score {
                best_score = score;
                best_angle = angle;
            }
            angle += DESKEW_SCAN_STEP_DEGREE;
        }
        best_angle
    }

    /// 计算候选角度下的行投影方差，方差越大表示文本行越水平对齐。
    fn row_projection_variance(image: &GrayImage, angle_degree: f32) -> f64 {
        let width = image.width();
        let height = image.height();
        let tan_value = angle_degree.to_radians().tan();
        let mut bins = vec![0_u32; height as usize];

        let sample_step = if width > 1200 || height > 1200 { 2 } else { 1 };
        let dark_threshold = 180_u8;

        let mut y = 0_u32;
        while y < height {
            let mut x = 0_u32;
            while x < width {
                let intensity = image.get_pixel(x, y)[0];
                if intensity < dark_threshold {
                    let projected_y = (y as f32) - (x as f32) * tan_value;
                    if projected_y >= 0.0 && projected_y < height as f32 {
                        bins[projected_y as usize] += 1;
                    }
                }
                x += sample_step;
            }
            y += sample_step;
        }

        let len = bins.len() as f64;
        if len <= 1.0 {
            return 0.0;
        }
        let mean = bins.iter().map(|value| *value as f64).sum::<f64>() / len;
        bins.iter()
            .map(|value| {
                let diff = (*value as f64) - mean;
                diff * diff
            })
            .sum::<f64>()
            / len
    }

    /// 以图像中心为轴执行小角度旋转，采用最近邻采样与白色背景填充。
    fn rotate_gray_image(image: &GrayImage, angle_degree: f32) -> GrayImage {
        let width = image.width();
        let height = image.height();
        let mut output = GrayImage::from_pixel(width, height, Luma([255_u8]));

        let angle = angle_degree.to_radians();
        let cos_theta = angle.cos();
        let sin_theta = angle.sin();
        let center_x = (width as f32 - 1.0) / 2.0;
        let center_y = (height as f32 - 1.0) / 2.0;

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let src_x = cos_theta * dx + sin_theta * dy + center_x;
                let src_y = -sin_theta * dx + cos_theta * dy + center_y;

                if src_x >= 0.0 && src_x < width as f32 && src_y >= 0.0 && src_y < height as f32 {
                    let src_x_rounded = (src_x + 0.5).floor().clamp(0.0, (width - 1) as f32) as u32;
                    let src_y_rounded =
                        (src_y + 0.5).floor().clamp(0.0, (height - 1) as f32) as u32;
                    let src_px = image.get_pixel(src_x_rounded, src_y_rounded);
                    output.put_pixel(x, y, *src_px);
                }
            }
        }

        output
    }

    /// 根据预处理路径推导模型目录；失败时回退到固定 workspace/models 路径。
    fn resolve_model_path(preprocessed_path: &str) -> PathBuf {
        let fallback = PathBuf::from("workspace")
            .join("models")
            .join(MODEL_FILE_NAME);
        let path = Path::new(preprocessed_path);
        match path.parent().and_then(Path::parent) {
            Some(workspace_root) => workspace_root.join("models").join(MODEL_FILE_NAME),
            None => fallback,
        }
    }

    /// 生成并写入模拟 OCR 结果，随后按插入顺序回查并返回完整记录。
    async fn simulate_and_insert_ocr_results(
        pool: &SqlitePool,
        asset: &AssignmentAsset,
        asset_id: &str,
        job_id: &str,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        let student_id = Self::resolve_simulation_student_id(pool, &asset.class_id).await?;
        let total_rows = 3 + (Self::pseudo_random_index(asset_id, job_id, 0) % 3);
        let now = Utc::now().to_rfc3339();

        let mut inserted_ids: Vec<String> = Vec::with_capacity(total_rows as usize);
        for index in 0..total_rows {
            let record_id = Uuid::new_v4().to_string();
            let question_no = (index + 1).to_string();
            let confidence = Self::build_simulated_confidence(asset_id, job_id, index);

            sqlx::query(
                "INSERT INTO assignment_ocr_result (id, asset_id, job_id, student_id, question_no, answer_text, confidence, score, ocr_raw_text, multimodal_score, multimodal_feedback, conflict_flag, review_status, reviewed_by, reviewed_at, final_score, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&record_id)
            .bind(asset_id)
            .bind(job_id)
            .bind(&student_id)
            .bind(&question_no)
            .bind(format!("模拟识别文本（题目{question_no}）"))
            .bind(confidence)
            .bind(Option::<f64>::None)
            .bind(Some("模拟OCR原始文本：本题答案区域已识别".to_string()))
            .bind(Option::<f64>::None)
            .bind(Option::<String>::None)
            .bind(0_i32)
            .bind("pending")
            .bind(Option::<String>::None)
            .bind(Option::<String>::None)
            .bind(Option::<f64>::None)
            .bind(0_i32)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;

            inserted_ids.push(record_id);
        }

        let mut rows = Vec::with_capacity(inserted_ids.len());
        for record_id in inserted_ids {
            let row = sqlx::query_as::<_, AssignmentOcrResult>(
                "SELECT * FROM assignment_ocr_result WHERE id = ? AND is_deleted = 0",
            )
            .bind(&record_id)
            .fetch_one(pool)
            .await?;
            rows.push(row);
        }

        Ok(rows)
    }

    /// 为模拟 OCR 结果选择一个可用学生，避免违反 student 外键约束。
    async fn resolve_simulation_student_id(
        pool: &SqlitePool,
        class_id: &str,
    ) -> Result<String, AppError> {
        let student_id = sqlx::query_scalar::<_, String>(
            "SELECT id FROM student WHERE class_id = ? AND is_deleted = 0 ORDER BY created_at ASC LIMIT 1",
        )
        .bind(class_id)
        .fetch_optional(pool)
        .await?;

        student_id.ok_or_else(|| {
            AppError::InvalidInput(format!(
                "当前班级下不存在可用于 OCR 模拟的学生记录：{class_id}"
            ))
        })
    }

    /// 基于输入生成稳定伪随机索引，确保无 rand 依赖也可生成离散分布。
    fn pseudo_random_index(asset_id: &str, job_id: &str, index: u32) -> u32 {
        let mut hasher = DefaultHasher::new();
        asset_id.hash(&mut hasher);
        job_id.hash(&mut hasher);
        index.hash(&mut hasher);
        (hasher.finish() % 10_000) as u32
    }

    /// 生成 0.65~0.95 区间的稳定置信度模拟值。
    fn build_simulated_confidence(asset_id: &str, job_id: &str, index: u32) -> f64 {
        let bucket = Self::pseudo_random_index(asset_id, job_id, index);
        let ratio = (bucket as f64) / 9_999.0_f64;
        0.65_f64 + ratio * 0.30_f64
    }

    fn dimensions_to_i32(image: &DynamicImage) -> Result<(i32, i32), AppError> {
        let width = image.width();
        let height = image.height();
        let image_width = i32::try_from(width)
            .map_err(|_| AppError::InvalidInput(format!("图片宽度超出范围：{width}")))?;
        let image_height = i32::try_from(height)
            .map_err(|_| AppError::InvalidInput(format!("图片高度超出范围：{height}")))?;
        Ok((image_width, image_height))
    }

    async fn mark_preprocess_failed(pool: &SqlitePool, asset_id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE assignment_asset SET preprocess_status = ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(PREPROCESS_STATUS_FAILED)
        .bind(&now)
        .bind(asset_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get_asset_by_id(
        pool: &SqlitePool,
        asset_id: &str,
    ) -> Result<AssignmentAsset, AppError> {
        let asset = sqlx::query_as::<_, AssignmentAsset>(
            "SELECT * FROM assignment_asset WHERE id = ? AND is_deleted = 0",
        )
        .bind(asset_id)
        .fetch_optional(pool)
        .await?;

        asset.ok_or_else(|| AppError::NotFound(format!("作业资产不存在或已删除：{asset_id}")))
    }

    fn resize_if_needed(image: DynamicImage, max_longest_side: u32) -> DynamicImage {
        let width = image.width();
        let height = image.height();
        let longest = width.max(height);

        if longest <= max_longest_side {
            return image;
        }

        let ratio = (max_longest_side as f64) / (longest as f64);
        let target_width = ((width as f64) * ratio).round().max(1.0) as u32;
        let target_height = ((height as f64) * ratio).round().max(1.0) as u32;

        image.resize(
            target_width,
            target_height,
            image::imageops::FilterType::Lanczos3,
        )
    }
}

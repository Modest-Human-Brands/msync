use std::{
    ffi::OsString,
    fs,
    io::BufReader,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use exif::{In, Reader as ExifReader, Tag};
use image::{
    imageops::{self, FilterType},
    DynamicImage, GenericImageView, RgbaImage,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tauri::ipc::InvokeError;

#[derive(Debug, Error)]
pub enum WatermarkError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("exif error: {0}")]
    Exif(#[from] exif::Error),

    #[error("rayon pool error: {0}")]
    RayonPool(String),

    #[error("unsupported media type: {0}")]
    UnsupportedMediaType(String),

    #[error("missing file name for path: {0}")]
    MissingFileName(PathBuf),

    #[error("ffmpeg failed for {path:?}: {stderr}")]
    FfmpegFailed { path: PathBuf, stderr: String },

    #[error("invalid option: {0}")]
    InvalidOption(String),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, WatermarkError>;

impl From<WatermarkError> for InvokeError {
    fn from(err: WatermarkError) -> Self {
        InvokeError::from(err.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatermarkOptions {
    /// Relative to the smallest side of the base image/video frame.
    pub scale_ratio: f32,

    /// Top-left placement in normalized coordinates [0.0, 1.0].
    pub position: (f32, f32),

    /// Relative to the smallest side of the base image/video frame.
    pub padding_ratio: f32,

    /// 0.0 to 1.0
    pub opacity: f32,
}

impl WatermarkOptions {
    pub fn validate(&self) -> Result<()> {
        if !(self.scale_ratio.is_finite() && self.scale_ratio > 0.0) {
            return Err(WatermarkError::InvalidOption(
                "scale_ratio must be > 0".into(),
            ));
        }
        if !(self.padding_ratio.is_finite() && self.padding_ratio >= 0.0) {
            return Err(WatermarkError::InvalidOption(
                "padding_ratio must be >= 0".into(),
            ));
        }
        if !(self.opacity.is_finite() && (0.0..=1.0).contains(&self.opacity)) {
            return Err(WatermarkError::InvalidOption(
                "opacity must be between 0.0 and 1.0".into(),
            ));
        }
        if !self.position.0.is_finite() || !self.position.1.is_finite() {
            return Err(WatermarkError::InvalidOption(
                "position must be finite".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VideoCodec {
    LibX264,
    LibX265,
    LibVpxVp9,
    LibAomAv1,
}

impl VideoCodec {
    fn as_ffmpeg_encoder(&self) -> &'static str {
        match self {
            VideoCodec::LibX264 => "libx264",
            VideoCodec::LibX265 => "libx265",
            VideoCodec::LibVpxVp9 => "libvpx-vp9",
            VideoCodec::LibAomAv1 => "libaom-av1",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoOptions {
    pub codec: Option<VideoCodec>,
    pub bitrate: Option<u32>,
    pub fps: Option<u32>,
    pub preserve_audio: bool,
}

impl Default for VideoOptions {
    fn default() -> Self {
        Self {
            codec: Some(VideoCodec::LibX264),
            bitrate: None,
            fps: None,
            preserve_audio: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkWatermarkReport {
    pub succeeded: Vec<PathBuf>,
    pub failed: Vec<(PathBuf, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkProgressEvent {
    pub current: usize,
    pub total: usize,
    pub path: String,
    pub status: String, // "done" | "error"
    pub error: Option<String>,
}

pub const WATERMARK_PROGRESS_EVENT: &str = "watermark-progress";
pub const WATERMARK_COMPLETE_EVENT: &str = "watermark-complete";

#[tauri::command]
fn add_watermarks(
    app: AppHandle,
    media_paths: Vec<PathBuf>,
    watermark_path: PathBuf,
    output_dir: PathBuf,
    watermark_options: WatermarkOptions,
    video_options: VideoOptions,
    thread_limit: Option<usize>,
) -> Result<BulkWatermarkReport> {
    watermark_options.validate()?;
    fs::create_dir_all(&output_dir)?;

    let watermark = Arc::new(load_image_with_exif_orientation(&watermark_path)?);
    let total = media_paths.len();
    let completed = Arc::new(AtomicUsize::new(0));

    let run = || {
        media_paths
            .into_par_iter()
            .map(|media_path| {
                let ext = media_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();

                let result = if is_image_ext(&ext) {
                    add_watermark_image(
                        &media_path,
                        watermark.as_ref(),
                        &output_dir,
                        &watermark_options,
                    )
                    .map_err(|e| (media_path.clone(), e.to_string()))
                } else if is_video_ext(&ext) {
                    add_watermark_video(
                        &media_path,
                        &watermark_path,
                        &output_dir,
                        &watermark_options,
                        &video_options,
                    )
                    .map_err(|e| (media_path.clone(), e.to_string()))
                } else {
                    Err((
                        media_path.clone(),
                        WatermarkError::UnsupportedMediaType(ext).to_string(),
                    ))
                };

                let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                let progress = match &result {
                    Ok(_) => WatermarkProgressEvent {
                        current,
                        total,
                        path: media_path.to_string_lossy().to_string(),
                        status: "done".into(),
                        error: None,
                    },
                    Err((_, err)) => WatermarkProgressEvent {
                        current,
                        total,
                        path: media_path.to_string_lossy().to_string(),
                        status: "error".into(),
                        error: Some(err.clone()),
                    },
                };

                let _ = app.emit(WATERMARK_PROGRESS_EVENT, progress);

                result
            })
            .collect::<Vec<_>>()
    };

    let outcomes = match thread_limit {
        Some(limit) if limit > 0 => {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(limit)
                .build()
                .map_err(|e| WatermarkError::RayonPool(e.to_string()))?;
            pool.install(run)
        }
        _ => run(),
    };

    let mut report = BulkWatermarkReport {
        succeeded: Vec::new(),
        failed: Vec::new(),
    };

    for outcome in outcomes {
        match outcome {
            Ok(path) => report.succeeded.push(path),
            Err((path, err)) => report.failed.push((path, err)),
        }
    }

    let _ = app.emit(WATERMARK_COMPLETE_EVENT, &report);

    Ok(report)
}

fn add_watermark_image(
    image_path: &Path,
    watermark: &DynamicImage,
    output_dir: &Path,
    options: &WatermarkOptions,
) -> Result<PathBuf> {
    options.validate()?;

    let base = load_image_with_exif_orientation(image_path)?;
    let (base_w, base_h) = base.dimensions();

    let wm = watermark.clone().to_rgba8();
    let (wm_w, wm_h) = wm.dimensions();

    let (target_w, target_h) = compute_watermark_size(base_w, base_h, wm_w, wm_h, options.scale_ratio);
    let resized_wm = resize_and_apply_opacity(&wm, target_w, target_h, options.opacity);

    let pad = ((base_w.min(base_h) as f32) * options.padding_ratio).round().max(0.0) as u32;
    let max_x = base_w.saturating_sub(target_w);
    let max_y = base_h.saturating_sub(target_h);

    let x = normalized_top_left_to_px(options.position.0, pad, max_x);
    let y = normalized_top_left_to_px(options.position.1, pad, max_y);

    let mut base_rgba = base.to_rgba8();
    imageops::overlay(&mut base_rgba, &resized_wm, x as i64, y as i64);

    let out_path = build_output_path(image_path, output_dir, "watermarked")?;
    save_dynamic_image(&DynamicImage::ImageRgba8(base_rgba), &out_path)?;
    Ok(out_path)
}

fn add_watermark_video(
    video_path: &Path,
    watermark_path: &Path,
    output_dir: &Path,
    options: &WatermarkOptions,
    video_options: &VideoOptions,
) -> Result<PathBuf> {
    options.validate()?;
    fs::create_dir_all(output_dir)?;

    let out_path = build_output_path(video_path, output_dir, "watermarked")?;
    let filter = build_ffmpeg_filter(options);

    let mut args: Vec<OsString> = Vec::new();

    args.push(OsString::from("-y"));
    args.push(OsString::from("-i"));
    args.push(video_path.as_os_str().to_os_string());

    args.push(OsString::from("-i"));
    args.push(watermark_path.as_os_str().to_os_string());

    args.push(OsString::from("-filter_complex"));
    args.push(OsString::from(filter));

    args.push(OsString::from("-map"));
    args.push(OsString::from("0:v:0"));
    args.push(OsString::from("-map"));
    args.push(OsString::from("[wmout]"));

    let codec = video_options
        .codec
        .as_ref()
        .map(VideoCodec::as_ffmpeg_encoder)
        .unwrap_or("libx264");

    args.push(OsString::from("-c:v"));
    args.push(OsString::from(codec));

    if let Some(bitrate) = video_options.bitrate {
        args.push(OsString::from("-b:v"));
        args.push(OsString::from(format!("{bitrate}")));
    }

    if let Some(fps) = video_options.fps {
        args.push(OsString::from("-r"));
        args.push(OsString::from(format!("{fps}")));
    }

    // Helps compatibility for common H.264/H.265 playback.
    args.push(OsString::from("-pix_fmt"));
    args.push(OsString::from("yuv420p"));

    if video_options.preserve_audio {
        args.push(OsString::from("-c:a"));
        args.push(OsString::from("copy"));
    } else {
        args.push(OsString::from("-an"));
    }

    args.push(out_path.as_os_str().to_os_string());

    run_ffmpeg(&args, video_path, &out_path)?;
    Ok(out_path)
}

fn build_ffmpeg_filter(options: &WatermarkOptions) -> String {
    let scale_ratio = options.scale_ratio;
    let opacity = options.opacity;
    let position_x = options.position.0.clamp(0.0, 1.0);
    let position_y = options.position.1.clamp(0.0, 1.0);
    let padding_ratio = options.padding_ratio;

    // FFmpeg computes the actual frame size dynamically.
    // Watermark is scaled against the smallest side, positioned from top-left,
    // and clamped inside the visible frame.
    format!(
        "[1:v]format=rgba,scale='min(main_w,main_h)*{scale_ratio}':-1[wm0];\
         [wm0]colorchannelmixer=aa={opacity}[wm1];\
         [0:v][wm1]overlay=x='min(max({position_x}*(main_w-overlay_w-{padding_ratio}*min(main_w,main_h)) + {padding_ratio}*min(main_w,main_h),0),main_w-overlay_w)':y='min(max({position_y}*(main_h-overlay_h-{padding_ratio}*min(main_w,main_h)) + {padding_ratio}*min(main_w,main_h),0),main_h-overlay_h)'[wmout]"
    )
}

fn run_ffmpeg(args: &[OsString], input: &Path, output: &Path) -> Result<()> {
    let output_result = Command::new("ffmpeg").args(args).output()?;

    if output_result.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output_result.stderr).to_string();
    Err(WatermarkError::FfmpegFailed {
        path: output.to_path_buf(),
        stderr: if stderr.is_empty() {
            format!("ffmpeg failed for {:?}", input)
        } else {
            stderr
        },
    })
}

fn load_image_with_exif_orientation(path: &Path) -> Result<DynamicImage> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let exif = ExifReader::new().read_from_container(&mut reader).ok();

    let mut img = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?;

    if let Some(exif) = exif {
        if let Some(orientation) = exif.get_field(Tag::Orientation, In::PRIMARY) {
            let value = orientation.value.get_uint(0).unwrap_or(1);
            img = apply_orientation(img, value);
        }
    }

    Ok(img)
}

fn apply_orientation(img: DynamicImage, orientation: u32) -> DynamicImage {
    match orientation {
        2 => DynamicImage::ImageRgba8(imageops::flip_horizontal(&img.to_rgba8())),
        3 => DynamicImage::ImageRgba8(imageops::rotate180(&img.to_rgba8())),
        4 => DynamicImage::ImageRgba8(imageops::flip_vertical(&img.to_rgba8())),
        5 => {
            let rgba = img.to_rgba8();
            let flipped = imageops::flip_horizontal(&rgba);
            DynamicImage::ImageRgba8(imageops::rotate90(&flipped))
        }
        6 => DynamicImage::ImageRgba8(imageops::rotate90(&img.to_rgba8())),
        7 => {
            let rgba = img.to_rgba8();
            let flipped = imageops::flip_horizontal(&rgba);
            DynamicImage::ImageRgba8(imageops::rotate270(&flipped))
        }
        8 => DynamicImage::ImageRgba8(imageops::rotate270(&img.to_rgba8())),
        _ => img,
    }
}

fn compute_watermark_size(
    base_w: u32,
    base_h: u32,
    wm_w: u32,
    wm_h: u32,
    scale_ratio: f32,
) -> (u32, u32) {
    let base_small = base_w.min(base_h).max(1) as f32;

    let mut target_w = (base_small * scale_ratio).round().max(1.0) as u32;
    let mut target_h = (((target_w as f32) / (wm_w.max(1) as f32)) * wm_h as f32)
        .round()
        .max(1.0) as u32;

    if target_w > base_w || target_h > base_h {
        let fit_scale = (base_w as f32 / wm_w.max(1) as f32)
            .min(base_h as f32 / wm_h.max(1) as f32);

        target_w = (wm_w as f32 * fit_scale).round().max(1.0) as u32;
        target_h = (wm_h as f32 * fit_scale).round().max(1.0) as u32;
    }

    (target_w.max(1), target_h.max(1))
}

fn resize_and_apply_opacity(
    watermark: &RgbaImage,
    target_w: u32,
    target_h: u32,
    opacity: f32,
) -> RgbaImage {
    let mut resized = imageops::resize(watermark, target_w, target_h, FilterType::Lanczos3);

    let alpha_scale = opacity.clamp(0.0, 1.0);
    for pixel in resized.pixels_mut() {
        let alpha = (pixel[3] as f32 * alpha_scale).round().clamp(0.0, 255.0) as u8;
        pixel[3] = alpha;
    }

    resized
}

fn normalized_top_left_to_px(position: f32, pad: u32, max_px: u32) -> u32 {
    let pos = position.clamp(0.0, 1.0);
    let available = max_px.saturating_sub(pad);
    let px = pad as f32 + pos * available as f32;
    px.round().clamp(0.0, max_px as f32) as u32
}

fn build_output_path(input: &Path, output_dir: &Path, suffix: &str) -> Result<PathBuf> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| WatermarkError::MissingFileName(input.to_path_buf()))?;

    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("");
    let filename = if ext.is_empty() {
        format!("{stem}_{suffix}")
    } else {
        format!("{stem}_{suffix}.{ext}")
    };

    Ok(output_dir.join(filename))
}

fn save_dynamic_image(img: &DynamicImage, out_path: &Path) -> Result<()> {
    img.save(out_path)?;
    Ok(())
}

fn is_image_ext(ext: &str) -> bool {
    matches!(
        ext,
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "tiff" | "gif" | "avif"
    )
}

fn is_video_ext(ext: &str) -> bool {
    matches!(
        ext,
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" | "flv" | "wmv" | "mpeg" | "mpg"
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_ffmpeg::init())
        .invoke_handler(tauri::generate_handler![add_watermarks])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

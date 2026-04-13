use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Semaphore;

#[derive(Serialize, Clone)]
struct OverlayProgressEvent {
    current: usize,
    total: usize,
    path: String,
    status: String,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct OverlayOptions {
    pub scale_ratio: f32,     // 0.0..1.0 of shortest side
    pub position: (f32, f32), // normalized x,y => 0..1
    pub padding_ratio: f32,   // 0.0..1.0 of shortest side
    pub opacity: f32,         // 0.0..1.0
}

#[derive(Serialize)]
pub struct BulkOverlayReport {
    pub succeeded: Vec<PathBuf>,
    pub failed: Vec<(PathBuf, String)>,
}

#[tauri::command]
pub async fn add_overlays(
    app: AppHandle,
    media_paths: Vec<PathBuf>,
    overlay_path: PathBuf,
    output_dir: PathBuf,
    overlay_options: OverlayOptions,
    thread_limit: Option<usize>,
) -> BulkOverlayReport {
    let limit = thread_limit.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        return BulkOverlayReport {
            succeeded: vec![],
            failed: media_paths
                .into_iter()
                .map(|p| (p, e.to_string()))
                .collect(),
        };
    }

    let semaphore = Arc::new(Semaphore::new(limit.max(1)));
    let mut tasks = Vec::new();

    for media in media_paths {
        let app = app.clone();
        let wm = overlay_path.clone();
        let out = output_dir.clone();
        let opts = sanitize_options(overlay_options);
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        tasks.push(tokio::spawn(async move {
            let _permit = permit;
            let result = add_overlay(&app, &media, &wm, &out, &opts).await;
            (media, result)
        }));
    }

    let total = tasks.len();
    let mut completed = 0usize;
    let mut report = BulkOverlayReport {
        succeeded: vec![],
        failed: vec![],
    };

    for task in tasks {
        if let Ok((path, result)) = task.await {
            completed += 1;
            let path_str = path.to_string_lossy().to_string();

            match result {
                Ok(_) => {
                    let _ = app.emit(
                        "overlay-progress",
                        OverlayProgressEvent {
                            current: completed,
                            total,
                            path: path_str,
                            status: "done".into(),
                            error: None,
                        },
                    );
                    report.succeeded.push(path);
                }
                Err(ref e) => {
                    let _ = app.emit(
                        "overlay-progress",
                        OverlayProgressEvent {
                            current: completed,
                            total,
                            path: path_str,
                            status: "error".into(),
                            error: Some(e.clone()),
                        },
                    );
                    report.failed.push((path, e.clone()));
                }
            }
        }
    }

    let _ = app.emit("overlay-complete", &report);
    report
}

async fn add_overlay(
    app: &AppHandle,
    input: &Path,
    wm_path: &Path,
    out_dir: &Path,
    opts: &OverlayOptions,
) -> Result<(), String> {
    let file_name = input.file_name().ok_or("Invalid filename")?;
    let output_path = out_dir.join(file_name);

    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let is_video = matches!(ext.as_str(), "mp4" | "mov" | "mkv" | "webm");

    let pad_expr = format!("(max(W,H)*{})", opts.padding_ratio);

    let mut filter = format!(
        "[0:v]setsar=1,split=2[bg_ref][bg]; \
         [1:v]setsar=1,format=rgba,colorchannelmixer=aa={opacity}[wm]; \
         [wm][bg_ref]scale=w='max(rw,rh)*{scale}':h='ow/a'[wm_scaled]; \
         [bg][wm_scaled]overlay=\
         x='(W-w-2*{pad})*{pos_x}+{pad}':\
         y='(H-h-2*{pad})*{pos_y}+{pad}'",
        opacity = opts.opacity,
        scale = opts.scale_ratio,
        pad = pad_expr,
        pos_x = opts.position.0,
        pos_y = opts.position.1
    );

    if is_video {
        filter.push_str(",format=yuv420p,pad=ceil(iw/2)*2:ceil(ih/2)*2");
    }

    let mut args = vec![
        "-y",
        "-loglevel",
        "error",
        "-nostdin",
        "-i",
        input.to_str().ok_or("Input path error")?,
        "-i",
        wm_path.to_str().ok_or("Watermark path error")?,
        "-filter_complex",
        &filter,
    ];

    if is_video {
        args.extend([
            "-c:v", "libx264", "-preset", "faster", "-crf", "23", "-c:a", "copy",
        ]);
    } else {
        args.extend(["-frames:v", "1", "-update", "1"]);
    }

    args.push(output_path.to_str().ok_or("Output path error")?);

    let output = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| format!("FFmpeg binary not found: {}", e))?
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to spawn FFmpeg process: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err_str = String::from_utf8_lossy(&output.stderr).to_string();
        if err_str.trim().is_empty() {
            let msg = format!(
                "SILENT CRASH on {:?}. Check if image paths are valid.",
                file_name
            );
            eprintln!("{}", msg);
            Err(msg)
        } else {
            eprintln!("FFMPEG ERROR on {:?}:\n{}", file_name, err_str);
            Err(err_str)
        }
    }
}

fn sanitize_options(opts: OverlayOptions) -> OverlayOptions {
    OverlayOptions {
        scale_ratio: opts.scale_ratio.clamp(0.01, 1.0),
        position: (
            opts.position.0.clamp(0.0, 1.0),
            opts.position.1.clamp(0.0, 1.0),
        ),
        padding_ratio: opts.padding_ratio.clamp(0.0, 1.0),
        opacity: opts.opacity.clamp(0.0, 1.0),
    }
}

use std::path::Path;

use image::RgbaImage;
use tauri::Emitter;
use walkdir::WalkDir;
use serde::{Deserialize, Serialize};


// 1. Define the Options struct to match the logic inside the function
// We keep this to group parameters, though we'll destructure them in the command signature
#[derive(Debug, Deserialize)]
struct WatermarkOptions {
    pub src_dir: String,
    pub dest_dir: String,
    pub watermark_path: String,
    pub position: String,
    pub size: u32,
    pub opacity: f32,
}

// 2. Define the Result struct to match the JS expectation
#[derive(Serialize)]
struct WatermarkResult {
    pub total: usize,
    pub errors: Vec<String>,
}

fn process_one(
    file_path: &std::path::Path,
    src_dir: &std::path::Path,
    dest_dir: &std::path::Path,
    watermark: &RgbaImage,
    options: &WatermarkOptions,
) -> Result<(), String> {
    // Open image as mutable directly to avoid cloning
    let mut img = image::open(file_path)
        .map_err(|e| e.to_string())?
        .to_rgba8();

    let (w, h) = img.dimensions();

    // 1. Calculate watermark dimensions based on width percentage
    let wm_w = (w * options.size / 100).max(1);
    let scale = wm_w as f32 / watermark.width() as f32;
    let wm_h = (watermark.height() as f32 * scale) as u32;

    // 2. Resize watermark
    let resized = image::imageops::resize(
        watermark, 
        wm_w, 
        wm_h, 
        image::imageops::FilterType::Lanczos3 // Corrected enum path
    );

    // 3. Calculate coordinates using saturating_sub to prevent "Subtract with Overflow"
    let padding = 10;
    let (dx, dy) = match options.position.as_str() {
        "top-left" => (padding, padding),
        "top-right" => (w.saturating_sub(wm_w).saturating_sub(padding), padding),
        "bottom-left" => (padding, h.saturating_sub(wm_h).saturating_sub(padding)),
        "center" => (
            w.saturating_sub(wm_w) / 2, 
            h.saturating_sub(wm_h) / 2
        ),
        _ => ( // Default bottom-right
            w.saturating_sub(wm_w).saturating_sub(padding), 
            h.saturating_sub(wm_h).saturating_sub(padding)
        ),
    };

    // 4. Blend watermark manually using your alpha logic
    for y in 0..wm_h {
        for x in 0..wm_w {
            let px = resized.get_pixel(x, y);
            // Apply both watermark pixel alpha and global opacity
            let alpha = (px[3] as f32 / 255.0) * options.opacity;

            let bx = dx + x;
            let by = dy + y;

            // Final bounds check before writing to pixel
            if bx < w && by < h {
                let base_px = img.get_pixel_mut(bx, by);
                for c in 0..3 {
                    base_px[c] = ((1.0 - alpha) * base_px[c] as f32 + alpha * px[c] as f32) as u8;
                }
                // Note: We keep the original image alpha or blend it if needed
            }
        }
    }

    // 5. Build destination path safely
    let relative = file_path.strip_prefix(src_dir).map_err(|e| e.to_string())?;
    let dest_path = dest_dir.join(relative);

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    img.save(dest_path).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn add_watermarks(
    window: tauri::WebviewWindow,
    // Note: To match JS invoke('...', { src_dir: .. }), 
    // we use individual arguments or a flattened struct.
    // Given your JS snippet, we define the arguments directly:
    src_dir: String,
    dest_dir: String,
    watermark_path: String,
    position: String,
    size: u32,
    opacity: f32,
) -> Result<WatermarkResult, String> {
    // Pack them into the options struct used by your existing logic
    let options = WatermarkOptions {
        src_dir,
        dest_dir,
        watermark_path,
        position,
        size,
        opacity,
    };

    let src_path = Path::new(&options.src_dir);
    let dest_path = Path::new(&options.dest_dir);

    // Load watermark
    let watermark = image::open(&options.watermark_path)
        .map_err(|e| format!("Failed to open watermark: {}", e))?
        .to_rgba8();

    let mut files = vec![];

    // Collect files
    for entry in WalkDir::new(src_path) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }

    let total = files.len();
    let mut errors = vec![];

    // Notify JS of total
    let _ = window.emit(
        "sidecar:event",
        serde_json::json!({
            "type": "total",
            "total": total
        }),
    );

    for (i, file_path) in files.iter().enumerate() {
        // Assume process_one is defined elsewhere in your logic
        let result = process_one(file_path, src_path, dest_path, &watermark, &options);

        match result {
            Ok(_) => {
                let _ = window.emit(
                    "sidecar:event",
                    serde_json::json!({
                        "type": "progress",
                        "processed": i + 1,
                        "total": total,
                        "file": file_path.file_name().unwrap_or_default().to_string_lossy()
                    }),
                );
            }
            Err(err) => {
                errors.push(err.clone());
                let _ = window.emit(
                    "sidecar:event",
                    serde_json::json!({
                        "type": "fileError",
                        "file": file_path.display().to_string(),
                        "message": err
                    }),
                );
            }
        }
    }

    let _ = window.emit(
        "sidecar:event",
        serde_json::json!({ "type": "done" }),
    );

    // Return the struct that matches JS { total: number, errors: string[] }
    Ok(WatermarkResult { total, errors })
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![add_watermarks])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

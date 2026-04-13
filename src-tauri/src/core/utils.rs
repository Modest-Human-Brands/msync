use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri_plugin_shell::ShellExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
    #[serde(rename = "photo")]
    Photo,
    #[serde(rename = "video")]
    Video,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub size: u64,
    pub resolution: String,
    pub aspect_ratio: String,
    pub bit_depth: String,
    // skip serializing if the Option is None to match `fps?: number`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub slug: String,
    // Using `rename` because `type` is a reserved keyword in Rust
    #[serde(rename = "type")]
    pub media_type: MediaType,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    pub metadata: MediaMetadata,
    pub path: String,
}

#[derive(serde::Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
}

#[derive(serde::Deserialize)]
struct FfprobeStream {
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    bits_per_raw_sample: Option<String>,
}

#[tauri::command]
pub async fn list_files(app: tauri::AppHandle, dir: String) -> Result<Vec<MediaItem>, String> {
    let mut result = Vec::new();
    let dir_path = Path::new(&dir);

    if !dir_path.exists() || !dir_path.is_dir() {
        return Err("Invalid directory path".to_string());
    }

    let entries = fs::read_dir(dir_path).map_err(|e| e.to_string())?;

    for entry_result in entries {
        let entry = entry_result.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();

        let media_type = match ext.as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "gif" => MediaType::Photo,
            "mp4" | "mov" | "mkv" | "webm" => MediaType::Video,
            _ => continue,
        };

        let metadata = get_media_metadata(&app, &path, &media_type).await?;

        let file_stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

        result.push(MediaItem {
            slug: slugify(&file_stem),
            media_type,
            title: file_stem,
            thumbnail_url: None,
            metadata: MediaMetadata {
                size,
                resolution: metadata.resolution,
                aspect_ratio: metadata.aspect_ratio,
                bit_depth: metadata.bit_depth,
                fps: metadata.fps,
            },
            path: path.to_string_lossy().into_owned(),
        });
    }

    Ok(result)
}

async fn get_media_metadata(
    app: &tauri::AppHandle,
    path: &Path,
    media_type: &MediaType,
) -> Result<MediaMetadata, String> {
    let output = app
        .shell()
        .sidecar("ffprobe")
        .map_err(|e| e.to_string())?
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,avg_frame_rate,bits_per_raw_sample,pix_fmt",
            "-of",
            "json",
            &path.to_string_lossy(),
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    let probe: FfprobeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse ffprobe JSON: {}", e))?;

    let v = probe.streams.first().ok_or("No video/image stream found")?;

    // 1. Resolution & Aspect Ratio
    let width = v.width.unwrap_or(0);
    let height = v.height.unwrap_or(0);
    let resolution = format!("{}x{}", width, height);

    // Simple GCD aspect ratio calculation
    let aspect_ratio = if width > 0 && height > 0 {
        let common = gcd(width, height);
        format!("{}:{}", width / common, height / common)
    } else {
        "N/A".to_string()
    };

    // 2. Bit Depth
    // ffprobe often returns null for bits_per_raw_sample on some codecs,
    // so we fallback to pix_fmt analysis or a default.
    let bit_depth = v
        .bits_per_raw_sample
        .clone()
        .map(|s| format!("{}-bit", s))
        .unwrap_or_else(|| "8-bit".to_string());

    // 3. FPS
    let fps = if let MediaType::Video = media_type {
        v.avg_frame_rate.as_ref().and_then(|s| {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() == 2 {
                let num: f64 = parts[0].parse().unwrap_or(0.0);
                let den: f64 = parts[1].parse().unwrap_or(1.0);
                if den != 0.0 {
                    Some(num / den)
                } else {
                    None
                }
            } else {
                s.parse::<f64>().ok()
            }
        })
    } else {
        None
    };

    Ok(MediaMetadata {
        size: 0, // Set in the main loop
        resolution,
        aspect_ratio,
        bit_depth,
        fps,
    })
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn slugify(name: &str) -> String {
    name.to_lowercase().replace(" ", "-").replace("_", "-")
}

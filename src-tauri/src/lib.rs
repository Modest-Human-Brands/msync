#[path = "./core/overlay.rs"]
mod overlay;
#[path = "./core/utils.rs"]
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            utils::list_files,
            overlay::add_overlays
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

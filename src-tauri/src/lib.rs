mod fs;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            fs::list_files,
            fs::read_file,
            fs::save_file,
            fs::rename_file,
            fs::sanitize_filename,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

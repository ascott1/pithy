mod config;
mod fs;
mod search;

use config::AppState;
use std::sync::Arc;
use tauri::menu::{AboutMetadata, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let (cfg, warning) =
                config::load_or_create().map_err(|e| e.to_string())?;
            let vault_dir = cfg.vault_dir.clone();
            app.manage(AppState {
                config: Arc::new(cfg),
                config_warning: warning,
            });

            let search_state = search::init_search(vault_dir)
                .map_err(|e| e.to_string())?;
            app.manage(search_state);

            let settings = MenuItemBuilder::new("Settings...")
                .id("settings")
                .accelerator("CmdOrCtrl+,")
                .build(app)?;

            let app_submenu = SubmenuBuilder::new(app, "Pithy")
                .about(Some(AboutMetadata::default()))
                .separator()
                .item(&settings)
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let edit_submenu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let menu = tauri::menu::MenuBuilder::new(app)
                .items(&[&app_submenu, &edit_submenu])
                .build()?;

            app.set_menu(menu)?;

            app.on_menu_event(move |app_handle, event| {
                if event.id() == settings.id() {
                    let _ = app_handle.emit("open-config", ());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            fs::list_files,
            fs::read_file,
            fs::save_file,
            fs::rename_file,
            fs::sanitize_filename,
            config::get_config_info,
            config::read_config_file,
            config::write_config_file,
            search::search_query,
            search::search_status,
            search::search_rebuild,
            search::list_tags,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

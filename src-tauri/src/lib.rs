mod config;
pub mod fs;
pub mod search;
mod titlebar;

use config::AppState;
use std::sync::{Arc, RwLock};
use tauri::menu::{AboutMetadata, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            let setup_start = std::time::Instant::now();

            let (cfg, warning) =
                config::load_or_create().map_err(|e| e.to_string())?;
            #[cfg(debug_assertions)]
            eprintln!("[pithy:perf] config load: {:?}", setup_start.elapsed());

            let vault_dir = cfg.vault_dir.clone();
            app.manage(AppState {
                config: Arc::new(RwLock::new(cfg)),
                config_warning: RwLock::new(warning),
            });

            let search_state = search::init_search(vault_dir)
                .map_err(|e| e.to_string())?;
            app.manage(search_state);

            #[cfg(debug_assertions)]
            eprintln!("[pithy:perf] setup total: {:?}", setup_start.elapsed());

            let settings = MenuItemBuilder::new("Settings...")
                .id("settings")
                .accelerator("CmdOrCtrl+,")
                .build(app)?;

            let new_note = MenuItemBuilder::new("New Note")
                .id("new-note")
                .accelerator("CmdOrCtrl+K")
                .build(app)?;

            let delete_note = MenuItemBuilder::new("Delete Note")
                .id("delete-note")
                .accelerator("CmdOrCtrl+Backspace")
                .build(app)?;

            let fullscreen = MenuItemBuilder::new("Enter Full Screen")
                .id("fullscreen")
                .accelerator("Ctrl+CmdOrCtrl+F")
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

            let file_submenu = SubmenuBuilder::new(app, "File")
                .item(&new_note)
                .item(&delete_note)
                .separator()
                .close_window()
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

            let view_submenu = SubmenuBuilder::new(app, "View")
                .item(&fullscreen)
                .build()?;

            let window_submenu = SubmenuBuilder::new(app, "Window")
                .minimize()
                .build()?;

            let help_submenu = SubmenuBuilder::new(app, "Help")
                .build()?;

            let menu = tauri::menu::MenuBuilder::new(app)
                .items(&[
                    &app_submenu,
                    &file_submenu,
                    &edit_submenu,
                    &view_submenu,
                    &window_submenu,
                    &help_submenu,
                ])
                .build()?;

            app.set_menu(menu)?;

            app.on_menu_event(move |app_handle, event| {
                if event.id() == settings.id() {
                    let _ = app_handle.emit("open-config", ());
                } else if event.id() == new_note.id() {
                    let _ = app_handle.emit("open-quick-switcher", ());
                } else if event.id() == delete_note.id() {
                    let _ = app_handle.emit("delete-note", ());
                } else if event.id() == fullscreen.id() {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        if let Ok(is_fs) = window.is_fullscreen() {
                            let _ = window.set_fullscreen(!is_fs);
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            fs::list_files,
            fs::read_file,
            fs::save_file,
            fs::delete_file,
            fs::rename_file,
            fs::sanitize_filename,
            fs::find_wikilink_references,
            fs::update_wikilink_references,
            fs::copy_image_to_assets,
            config::get_config_info,
            config::read_config_file,
            config::write_config_file,
            config::update_config,
            config::list_themes,
            search::search_query,
            search::search_status,
            search::search_rebuild,
            search::list_tags,
            titlebar::set_titlebar_opacity,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

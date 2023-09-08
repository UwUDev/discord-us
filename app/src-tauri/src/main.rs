// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
mod state;
mod database;
mod commands;

use tauri::{Manager, State, command};
use crate::database::{Database, get_items, get_options, set_options, get_option};
use crate::state::{AppState, AppInitializer, WindowManager};
use crate::settings::{get_settings, save_settings, Settings};
use crate::commands::{handle_file_drop, open_window, pick_file, upload_file};


fn main() {
    tauri::Builder::default()
        .manage(AppState {
            settings: Default::default(),
            database: Default::default(),

            window_manager: WindowManager::new(),
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,

            get_items,
            get_options,
            set_options,
            get_option,

            handle_file_drop,
            open_window,
            pick_file,
            upload_file,

        ])
        .plugin(tauri_plugin_context_menu::init())
        .setup(|app| {
            let handle = app.handle();

            let state: State<AppState> = app.state();

            *state.settings.lock().unwrap() = Some(Settings::init(&handle));

            *state.database.lock().unwrap() = Some(Database::init(&handle));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
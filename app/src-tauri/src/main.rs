// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
mod state;
mod database;

use tauri::{Manager, State};
use crate::state::{AppState, AppInitializer};
use crate::settings::{get_settings, save_settings, Settings};

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            settings: Default::default()
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings
        ])
        .plugin(tauri_plugin_context_menu::init())
        .setup(|app| {
            let handle = app.handle();

            let state : State<AppState> = app.state();

            *state.settings.lock().unwrap() = Some(Settings::init(&handle));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
mod state;
mod database;
mod commands;
mod manager;

use tauri::{Manager, State, command, RunEvent};
use crate::database::{Database, get_items, get_item, get_options, set_options, get_option};
use crate::state::{AppState, AppExit, AppInitializer, WindowManager};
use crate::settings::{get_settings, save_settings, Settings};
use crate::commands::{handle_file_drop, open_window, pick_file, upload_file,save_file_picker,export_waterfall, delete_items};


fn main() {
    tauri::Builder::default()
        .manage(AppState {
            settings: Default::default(),
            database: Default::default(),

            window_manager: WindowManager::new(),

            manager: Default::default(),
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,

            get_items,
            get_item,
            get_options,
            set_options,
            get_option,

            handle_file_drop,
            open_window,
            pick_file,
            upload_file,
            save_file_picker,
            export_waterfall,

            delete_items

        ])
        .plugin(tauri_plugin_context_menu::init())
        .setup(|app| {
            let handle = app.handle();

            let state: State<AppState> = app.state();

            *state.settings.lock().unwrap() = Some(Settings::init(&handle));

            *state.database.lock().unwrap() = Some(Database::init(&handle));

            *state.manager.lock().unwrap() = Some(crate::manager::Manager::init(&handle));

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error building app")
        .run(|app_handle, event| match event {
            RunEvent::ExitRequested { api, .. } => {
                println!("exit requested");

                let state: State<AppState> = app_handle.state();

                let mut manager = state.manager.lock().unwrap();

                manager.as_mut().unwrap().exit(app_handle);
            }
            _ => {
               // println!("event {:?}", event);
            }
        });
}
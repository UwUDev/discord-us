use std::fs::{read_to_string, write};
use std::path::PathBuf;
use tauri::{AppHandle, command, Manager, State, Window};
use crate::state::{AppState, AppDirectory, AppInitializer};

#[derive(Clone)]
pub struct Settings {
    file_path: PathBuf,
}

impl AppInitializer for Settings {
    fn init (app_handle: &AppHandle) -> Self {
        let file_path = app_handle.get_app_data_dir().join("settings.json");

        println!("Settings file path: {:?}", file_path);

        Self {
            file_path,
        }
    }
}

impl Settings {
    fn read_settings(&self) -> Option<String> {
        let contents = read_to_string(&self.file_path);

        contents.ok()
    }

    fn write_settings(&self, contents: &String) {
        write(&self.file_path, contents).unwrap();
    }
}

#[command]
pub fn get_settings(app_state: State<'_, AppState>) -> Option<String> {
    app_state.settings.lock().unwrap().as_ref().map(|settings| settings.read_settings()).flatten()
}

#[command]
pub fn save_settings(app_handle: AppHandle, window: Window, settings: String) {
    let state : State<AppState> = app_handle.state();

    // to think: use cache + delayed writes
    state.settings.lock().unwrap().as_ref().map(|s| s.write_settings(&settings));

    for (label, w) in app_handle.windows().iter() {
        if label != window.label() {
            app_handle.emit_to(label, "settings-updated", &settings).unwrap();
        }
    }
}
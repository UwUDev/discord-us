use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::settings::{Settings};
use tauri::{AppHandle};

pub struct AppState {
    pub settings: Mutex<Option<Settings>>,
}

pub trait AppDirectory {
    fn get_app_data_dir(&self) -> PathBuf;
}

impl AppDirectory for AppHandle {
    fn get_app_data_dir(&self) -> PathBuf {
        let path = self.path_resolver().app_data_dir().expect("The app data directory should exist.");

        create_dir_all(&path).expect("The app data directory should be created.");

        path
    }
}

pub trait AppInitializer {
    fn init(app_handle: &AppHandle) -> Self;
}
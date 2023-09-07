use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::Mutex;

use crate::settings::{Settings};
use crate::database::{Database};
use tauri::{AppHandle};


pub struct AppState {
    pub settings: Mutex<Option<Settings>>,

    pub database: Mutex<Option<Database>>,

    pub window_manager: WindowManager,
}

pub struct WindowManager {
    unique_counter: AtomicU32,
}

impl WindowManager {
    pub fn new () -> Self {
        Self {
            unique_counter: AtomicU32::new(0),
        }
    }

    pub fn unique_window_label(&self) -> String {
        format!("window-{}", self.unique_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
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
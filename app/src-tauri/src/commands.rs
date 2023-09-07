use std::fs;
use tauri::{command, AppHandle};
use fs::metadata;
use std::ffi::OsStr;
use std::fmt::format;
use std::path::Path;

#[command]
pub fn handle_file_drop(app_handle: AppHandle, path: &str) {
    println!("File dropped: {}", path);

    // if directory => not supported (for the moment)
    let path = Path::new(path);

    if !path.exists() || path.is_dir() {
        return;
    }

    // get extension
    let page = path.extension().map(|ext| {
        match ext.to_str() {
            Some("waterfall") => ("download", "Download file"),
            _ => ("upload", "Upload file")
        }
    });

    if let Some((endpoint, label)) = page {
        let p = format!("index.html?page={}&fpath={}", endpoint, path.to_str().unwrap());

        let window = tauri::WindowBuilder::new(
            &app_handle,
            "local",
            tauri::WindowUrl::App(p.into()),
        ).build().unwrap();
    }
}
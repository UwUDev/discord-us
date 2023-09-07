use std::fs;
use tauri::{command, AppHandle, Manager, State, Window};
use tauri::api::dialog::{FileDialogBuilder};
use std::path::Path;
use serde::Serialize;
use crate::state::AppState;


#[command]
pub async fn handle_file_drop(app_handle: AppHandle, path: String) {
    println!("File dropped: {}", path);

    // if directory => not supported (for the moment)
    let path = Path::new(path.as_str());

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

    if let Some((endpoint, title)) = page {
        let p = format!("index.html?path={}&fpath={}", endpoint, path.to_str().unwrap());

        open_window(app_handle, p, title.to_string()).await;
    }
}

#[command]
pub async fn open_window(app_handle: AppHandle, url: String, title: String) {
    let state: State<'_, AppState> = app_handle.state();

    let label = state.window_manager.unique_window_label();

    let window = tauri::WindowBuilder::new(
        &app_handle,
        label, /* the unique window label */
        tauri::WindowUrl::App(url.into())
    ).closable(true)
        .title(title)
        .center()
        .build().unwrap();
}

#[derive(Serialize, Clone)]
pub struct PickFileResponse {
    callback: String,
    path: String
}

#[command]
pub fn pick_file (window: Window, cb: String) {
    println!("Pick file {:?} => {:?}", window.label(), cb);

    FileDialogBuilder::default().pick_file(move |path| match path {
        Some(p) => {
            window.emit("file-picked", PickFileResponse { callback:cb, path: p.to_str().unwrap().to_string() }).unwrap();
        },
        _ => {}
    })
}
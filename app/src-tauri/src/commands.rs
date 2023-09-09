use std::fmt::Debug;
use std::rc::Rc;
use tauri::{command, AppHandle, Manager, State, Window};
use tauri::api::dialog::{FileDialogBuilder};
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::manager::{UploadStatus, UploadingItem};
use crate::state::AppState;
use crate::database::{ItemStatus, _get_option, _get_item};
use rand::{distributions::Alphanumeric, Rng};
use discord_us::common::{ResumableFileUpload, Subscription, Waterfall, FileWritable};
use discord_us::uploader::{WaterfallExporter};

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

        open_window(app_handle, p, title.to_string(), None).await;
    }
}

#[command]
pub async fn open_window(app_handle: AppHandle, url: String, title: String, label: Option<String>) {
    let state: State<'_, AppState> = app_handle.state();

    let label = label.or_else(|| Some(state.window_manager.unique_window_label())).unwrap();

    let window = tauri::WindowBuilder::new(
        &app_handle,
        label, /* the unique window label */
        tauri::WindowUrl::App(url.into()),
    ).closable(true)
        .visible(false)
        .title(title)
        .center()
        .build();

    if let Ok(w) = window {
        // window created!
    }
}

#[derive(Serialize, Clone)]
pub struct PickFileResponse {
    callback: String,
    path: String,
}

#[command]
pub fn pick_file(window: Window, cb: String) {
    println!("Pick file {:?} => {:?}", window.label(), cb);

    FileDialogBuilder::default().pick_file(move |path| match path {
        Some(p) => {
            window.emit("file-picked", PickFileResponse { callback: cb, path: p.to_str().unwrap().to_string() }).unwrap();
        }
        _ => {}
    })
}

#[command]
pub fn save_file_picker(window: Window, cb: String, extensions: Vec<&str>) {
    println!("Pick file {:?} => {:?}", window.label(), cb);

    FileDialogBuilder::default().add_filter("extensions", &extensions).save_file(move |path| match path {
        Some(p) => {
            window.emit("file-picked", PickFileResponse { callback: cb, path: p.to_str().unwrap().to_string() }).unwrap();
        }
        _ => {}
    })
}

#[derive(Deserialize, Clone)]
pub struct UploadFilePayload {
    file_path: String,
    thread_count: u32,

    password: Option<String>,

    container_size: Option<u32>,
}


pub fn create_random_password(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

#[command]
pub fn upload_file(app_handle: AppHandle, payload: UploadFilePayload) -> Result<(), String> {
    // we need to register the file
    // in database
    // then call a "resume function" on download manager

    let state: State<'_, AppState> = app_handle.state();

    let mut database_guard = state.database.lock().unwrap();

    let mut database = database_guard.as_mut().unwrap();

    let mut container_size: u32;

    if let Some(account_container_size) = _get_option(&database, &"account_type".into()) {
        let subscription: Subscription = account_container_size.as_str().into();
        container_size = subscription.get_max_chunk_upload_size() as u32;
    } else {
        // return Err
        return Err("No subscription found".into());
    }

    if let Some(payload_container_size) = &payload.container_size {
        if *payload_container_size > container_size {
            // return Err
            return Err("Container size is too big".into());
        }
        container_size = *payload_container_size;
    }

    let mut stmt = database.connection
        .prepare("INSERT INTO items (name, status, password, user_password, thread_count,file_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .unwrap();

    let mut params_values: Vec<rusqlite::types::Value> = Vec::with_capacity(6);

    let user_password = payload.password.is_some();

    params_values.push(payload.file_path.clone().into());
    params_values.push(ItemStatus::UPLOADING.to_code().into());
    params_values.push(payload.password.or_else(|| Some(create_random_password(16))).unwrap().into());
    params_values.push(user_password.into());
    params_values.push(payload.thread_count.into());
    params_values.push(payload.file_path.into());

    let id = stmt.insert(rusqlite::params_from_iter(params_values)).unwrap();

    drop(stmt);

    let item = _get_item(&database, id as i32).unwrap();

    app_handle.emit_all("push_item", item.clone()).unwrap();

    let mut token: String;
    let mut channel_id: u64;

    if let Some(token_option) = _get_option(&database, &"upload_token".into()) {
        token = token_option;
    } else {
        return Err("No discord token found".into());
    }

    if let Some(channel_id_option) = _get_option(&database, &"channel_id".into()).map(|v| v.parse::<u64>().unwrap()) {
        channel_id = channel_id_option;
    } else {
        return Err("No discord channel id found".into());
    }

    drop(database_guard);

    let mut manager_guard = state.manager.lock().unwrap();
    let mut manager = manager_guard.as_mut().unwrap();

    let uploading_item = UploadingItem::from_item(
        item,
        &token,
        channel_id,
        Some(container_size),
    );


    let id = manager.register_uploading_item(uploading_item).unwrap();

    if let Err(err) = manager.resume_upload(id, &app_handle) {
        return Err(err.into());
    }

    Ok(())
}

#[command]
pub fn export_waterfall(app_handle: AppHandle, item_id: i32, waterfall_path: String, password: Option<String>) -> Result<(), &'static str> {
    let app_state: State<'_, AppState> = app_handle.state();

    let database_guard = app_state.database.lock().unwrap();
    let database = database_guard.as_ref().unwrap();
    println!("Fetching {}", item_id);
    if let Ok(item) = _get_item(database, item_id) {
        println!("Exporting item {}", item.id);
        if item.status.to_code() != ItemStatus::DONE.to_code() || item.resume_data.is_none() {
            return Err("Item is not uploaded");
        }

        if let Ok(resume_session) = serde_json::from_str::<ResumableFileUpload>(&item.resume_data.unwrap()) {

            let mut waterfall = match password {
                Some(p) => resume_session.export_waterfall_with_password(p),
                None => resume_session.export_waterfall()
            };
            println!("write_to_file {}", waterfall_path);
            waterfall.write_to_file(waterfall_path);

            return Ok(());
        }
    }

    Err("Cannot save waterfall")
}
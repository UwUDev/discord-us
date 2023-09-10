use std::collections::{HashMap};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::AtomicBool;
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, State};
use discord_us::common::ResumableFileUpload;
use discord_us::uploader::{FileUploadArguments, FileUploader, ResumableUploader, Uploader};
use discord_us::signal::{PartProgression, Signal};
use crate::database::{_update_item, _get_option, Item, ItemStatus, Database, _get_items_with_status, notify_item_updated};
use crate::state::{AppInitializer, AppState, AppExit};

pub struct UploadingItem {
    arguments: FileUploadArguments,

    resume_session: Option<ResumableFileUpload>,

    item: Item,

    container_size: u32,

    pub upload_status: Option<UploadStatus>,

    uploader: Option<FileUploader>,
}

impl UploadingItem {
    pub fn from_item(item: Item, token: &String, channel_id: u64, s: Option<u32>) -> Self {
        let mut args = FileUploadArguments::new(
            (&item.password).clone(),
            token.clone(),
            channel_id,
        );

        let resume_session: Option<ResumableFileUpload> = (&item.resume_data).clone().map(|data| {
            serde_json::from_str(&data).unwrap()
        });

        println!("Item: {:?}", &item);

        let container_size = s.unwrap_or_else(|| resume_session.as_ref().unwrap().container_size);

        Self {
            arguments: args,
            resume_session,
            item,
            upload_status: None,
            container_size,

            uploader: None,
        }
    }

    pub fn id(&self) -> i32 {
        return self.item.id;
    }
}

pub struct UploadStatus {
    pub total_size: u64,
    pub progression: PartProgression<u64>,
}

impl UploadStatus {}

pub struct Manager {
    uploads: HashMap<i32, UploadingItem>,
}

unsafe impl Send for Manager {}

impl AppInitializer for Manager {
    fn init(app_handle: &AppHandle) -> Self {
        Self {
            uploads: HashMap::new(),
        }
    }
}

impl Manager {
    pub fn register_uploading_item(&mut self, item: UploadingItem) -> Option<i32> {
        let id = item.id();

        if self.uploads.contains_key(&id) {
            return None;
        }

        self.uploads.insert(id, item);

        Some(id)
    }

    pub fn get_item(&self, id: i32) -> Option<&UploadingItem> {
        self.uploads.get(&id)
    }

    pub fn resume_upload(&mut self, id: i32, handle: &AppHandle) -> Result<(), &str> {
        let item = self.uploads.get_mut(&id);

        if item.is_none() {
            return Err("Item not found");
        }

        let item = item.unwrap();

        if item.uploader.is_some() {
            return Err("Item is not paused !");
        }

        let signal = PartProgression::new();

        item.arguments.with_signal(&signal);

        let path = item.item.file_path.clone();
        let container_size = item.container_size;
        let threads = item.item.thread_count;

        let mut uploader = if let Some(resume_session) = &item.resume_session {
            FileUploader::from_resume_session(resume_session.clone()).unwrap()
        } else {
            FileUploader::new_with_threads_count(path, container_size, threads)
        };

        let size = uploader.upload(item.arguments.clone());

        let upload_status = UploadStatus {
            total_size: size,
            progression: signal.clone(),
        };

        let thread_pool = uploader.get_thread_pool();

        let running_state = uploader.get_running_state();

        item.uploader = Some(uploader);
        item.upload_status = Some(upload_status);

        let mut upload_manager_thread = UploadManagerThread {
            signal: signal.clone(),
            app_handle: handle.clone(),
            total: size,
            id: item.id(),
            running: running_state,
        };

        let app_handle = handle.clone();

        std::thread::spawn(move || {
            upload_manager_thread.run(); //<- upload monitor

            thread_pool.join(); // <- wait for threads to finish

            use tauri::Manager;

            println!("UploadManagerThread finished id={}", id);

            // retrieve manager
            let state: State<'_, AppState> = app_handle.state();
            let mut manager_guard = state.manager.lock().unwrap();
            let mut manager = manager_guard.as_mut().unwrap();

            if let Ok((resume_data, progress_data)) = manager.export_resume_data(&id) {
                println!("UploadManagerThread export_resume_data: {:?}, {:?}", resume_data, progress_data);

                manager.uploads.remove(&id); // remove item from manager

                drop(manager_guard); // we don't need manager anymore thus we release the lock

                // update database

                let database_guard = state.database.lock().unwrap();
                let database = database_guard.as_ref().unwrap();

                _update_item(database, id, progress_data, resume_data, ItemStatus::DONE);

                notify_item_updated(database, id, &app_handle);
            }
        });

        Ok(())
    }

    pub fn export_resume_data(&mut self, id: &i32) -> Result<(Option<String>, Option<String>), &str> {
        Self::_export_resume_data(*id, self.uploads.get(id))
    }

    /// export resume data for an item
    /// first element is the resume data (json) (optional)
    /// and second element is the progress data (json)
    pub fn _export_resume_data(id: i32, item: Option<&UploadingItem>) -> Result<(Option<String>, Option<String>), &'static str> {
        return match item {
            Some(item) => {
                let resume_data: Option<String> = item.uploader.as_ref().map(|uploader| {
                    serde_json::to_string(&uploader.export_resume_session()).unwrap()
                });

                let progress_data: Option<String> = item.uploader.as_ref().map(|uploader| {
                    let ranges: Vec<[u64; 2]> = uploader.get_uploaded_ranges().iter().map(|range| {
                        [range.range_start, range.range_end]
                    }).collect();

                    let progress = ranges.iter().fold(0, |acc, range| {
                        acc + (range[1] - range[0])
                    });

                    let total = uploader.get_total_upload_size();

                    json!({
                        "ranges": ranges,
                        "progress": progress,
                        "total": total
                    }).to_string()
                });

                //let progression_data = uploader.

                Ok((resume_data, progress_data))
            }
            None => Err("Item not found"),
        };
    }

    fn _pause_upload(database: &mut Database, id: &i32, item: &UploadingItem) -> Result<(), &'static str> {
        match item.uploader.as_ref() {
            Some(uploader) => {
                println!("Stopping uploader for {}", *id);
                uploader.set_running(false);

                Self::_export_resume_data(*id, Some(item)).map(|(resume_data, progress_data)| {
                    println!("Saving {:?} {:?}", resume_data, progress_data);
                    _update_item(database, *id, progress_data, resume_data, ItemStatus::UPLOADING);
                })
            }
            None => Err("Item not being uploaded")
        }
    }

    pub fn pause_upload(&mut self, database: &mut Database, id: &i32) -> Result<(), &'static str> {
        let item = self.uploads.remove(&id);

        match item {
            Some(item) => Self::_pause_upload(database, id, &item),
            None => Err("Item not found")
        }
    }

    fn save_all(&mut self, database: &mut Database) {
        // save all current uploading sessions
        println!("Saving {} items", self.uploads.len());

        for (id, item) in self.uploads.iter() {
            Self::_pause_upload(database, id, item).unwrap();
        }

        self.uploads.clear();
    }

    // pub fn reimport_all_uploads (&mut self, ) {
    //     use tauri::Manager;
    //
    //     let state: State<'_, AppState> = app_handle.state();
    //     let database_guard = state.database.lock().unwrap();
    //     let database = database_guard.as_ref().unwrap();
    //
    //     let token = _get_option(&database, &"upload_token".into());
    //     let channel_id = _get_option(&database, &"channel_id".into()).map(|v| v.parse::<u64>().ok()).flatten();
    //
    //     if token.is_some() && channel_id.is_some() {
    //         let token = token.unwrap();
    //         let channel_id = channel_id.unwrap();
    //         for item in _get_items_with_status(&database, ItemStatus::UPLOADING).unwrap().iter() {
    //             self.register_uploading_item(
    //                 UploadingItem::from_item(
    //                     item.clone(),
    //                     &token,
    //                     channel_id,
    //                     None,
    //                 )
    //             );
    //
    //             // checked whether was paused or not...
    //             // self.resume_upload(item.id, app_handle).unwrap();
    //         }
    //     }
    // }
}

impl AppExit for Manager {
    fn exit(&mut self, app_handle: &AppHandle) {
        use tauri::Manager;

        // export all current uploading sessions
        let state: State<'_, AppState> = app_handle.state();

        let mut database_guard = state.database.lock().unwrap();

        let mut database = database_guard.as_mut().unwrap();

        self.save_all(database);
    }
}

#[derive(Serialize, Clone)]
pub struct UploadProgressEvent {
    id: i32,
    progress: u64,
    total: u64,

    ranges: Vec<[u64; 2]>,
}

struct UploadManagerThread {
    running: Arc<RwLock<bool>>,
    signal: PartProgression<u64>,
    app_handle: AppHandle,
    total: u64,
    id: i32,
}

unsafe impl Send for UploadManagerThread {}

impl UploadManagerThread {
    fn is_running(&self) -> bool {
        *self.running.read().unwrap()
    }

    fn run(&mut self) {
        while self.is_running() {
            println!("UploadManagerThread running (sleep for 100ms)");
            std::thread::sleep(std::time::Duration::from_millis(100)); // sleep for 100ms

            self.signal.retrim_ranges();

            let progress = self.signal.get_total();
            let data = self.signal.get_data();

            let mut ranges: Vec<[u64; 2]> = Vec::with_capacity(data.len());

            for range in data.iter() {
                ranges.push([range.range_start, range.range_end]);
            }
            println!("UploadManagerThread progress: {} {}/{} ({:.2}%)", progress, progress, self.total, (progress as f64 / self.total as f64) * 100.0);

            use tauri::Manager;

            self.app_handle.emit_all("upload_progress", UploadProgressEvent {
                progress,
                total: self.total,
                ranges,
                id: self.id,
            }).unwrap();

            if progress == self.total {
                println!("UploadManagerThread 100%");
                break;
                //(self.on_end)(&self.app_handle);
            }
        }
    }
}
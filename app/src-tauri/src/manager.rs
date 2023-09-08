use std::sync::Mutex;
use discord_us::common::ResumableFileUpload;
use discord_us::uploader::{FileUploadArguments, FileUploader};
use discord_us::signal::{PartProgression};
use crate::database::{Item};

pub struct UploadingItem {
    arguments: FileUploadArguments,
    path: String,

    resume_session: Option<ResumableFileUpload>,

    is_paused: bool,

    thread_count: u32,
    file_path: String,
}

impl UploadingItem {
    // pub fn from_item(item: &Item, token: &String, channel_id: u64) -> Self {
    //     let mut args = FileUploadArguments::new(
    //         (&item.password).clone(),
    //         token.clone(),
    //         channel_id,
    //     );
    //
    //     let resume_session = (&item.resume_data).clone().map(|data| {
    //         serde_json::from_str(&data).unwrap()
    //     });
    //
    //     let signal = PartProgression::new();
    //
    //     args.with_signal(&signal);
    //
    //     Self {
    //         arguments: args,
    //         is_paused: true,
    //         resume_session,
    //
    //         path: (&item.name).clone(),
    //         thread_count: item.thread_count,
    //         file_path: (&item.file_path).clone(),
    //     }
    // }
}

pub struct Manager {
    uploads: Vec<UploadingItem>,
}

impl Manager {
    pub fn register_uploading_item(&mut self, item: UploadingItem) -> usize {
        self.uploads.push(item);
        self.uploads.len() - 1
    }

    pub fn resume(&mut self, index: usize) -> Result<(), &str> {
        if !self.uploads[index].is_paused {
            return Err("Already resumed");
        }

        //let item = self.uploads[index];

        //let mut uploader = FileUploader::new(item.path.clone(), item.arguments.clone());

        self.uploads[index].is_paused = false;


        Ok(())
    }
}
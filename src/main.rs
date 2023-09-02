mod utils;
mod uploader;
mod database;
mod downloader;
mod http_client;

use uploader::{FileUploader, Uploader};
use crate::utils::{create_trash_dir};


fn main() {
    create_trash_dir();
    let token = String::from("...");
    let channel_id: u64 = 0;

    FileUploader::new_with_threads_count("fichier".to_string(), 24 * 1024 * 1024, 1)
        .upload("password".to_string(), token, channel_id);
}


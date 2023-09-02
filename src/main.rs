mod utils;
mod uploader;
mod database;
mod downloader;
mod http_client;
mod common;

use std::fs::File;
use std::io::{Read, Write};
use uploader::{FileUploader, Uploader};
use crate::common::{FileWritable, Waterfall};
use crate::downloader::{Downloader, FileDownloader, WaterfallDownloader};
use crate::uploader::WaterfallExporter;
use crate::utils::{create_trash_dir};


fn main() {
    create_trash_dir();
    let token = String::from("...");
    let channel_id: u64 = 0;

    let password = "password".to_string();

    // let downloader = FileDownloader::from_waterfall(Waterfall::from_file("inoxtag.waterfall".to_string()));
    //
    // downloader.download_file("inoxtag.mov".to_string());

    let waterfall = FileUploader::new_with_threads_count("trash/IMG_0577.mov".to_string(), 24 * 1024 * 1024, 1)
        .upload(password.clone(), token, channel_id)
        .export_waterfall_with_password(password.clone());

    waterfall.write_to_file("inoxtag.waterfall".to_string());
}


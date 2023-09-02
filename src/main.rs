mod utils;
mod uploader;
mod database;
mod downloader;
mod http_client;
mod common;

use std::fs::File;
use std::io::Write;
use uploader::{FileUploader, Uploader};
use crate::common::Waterfall;
use crate::downloader::{FileDownloader, WaterfallDownloader};
use crate::uploader::WaterfallExporter;
use crate::utils::{create_trash_dir};


fn main() {
    create_trash_dir();
    let token = String::from("...");
    let channel_id: u64 = 0;

    let password = "password".to_string();

    let waterfall = serde_json::from_str::<Waterfall>(
        &std::fs::read_to_string("cool.waterfall").unwrap()
    ).unwrap();

    waterfall.download(password.clone());

    //println!("Chunks count: {:?}", chunks);

    // let waterfall = FileUploader::new_with_threads_count("cargo.toml".to_string(), 24 * 1024 * 1024, 1)
    //     .upload(password.clone(), token, channel_id)
    //     .export_waterfall_with_password(password.clone());
    //
    // println!("{:?}", serde_json::to_string(&waterfall).unwrap());
    //
    // let mut file = File::create("cool.waterfall").unwrap();
    // file.write_all(serde_json::to_string_pretty(&waterfall).unwrap().as_bytes()).unwrap();
}


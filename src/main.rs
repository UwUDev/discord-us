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

    let waterfall = serde_json::from_str::<common::Waterfall>(
        &std::fs::read_to_string("cool.waterfall").unwrap()
    ).unwrap();

    let container = waterfall.clone().containers.get(0).unwrap().clone();

    let downloader = FileDownloader::from_waterfall(waterfall);

    let download_container = downloader.get_container_downloader(container);

    let chunks = download_container.get_chunks(0, 1);

    println!("Chunks count: {:?}", chunks);

    // let waterfall = FileUploader::new_with_threads_count("cargo.toml".to_string(), 24 * 1024 * 1024, 1)
    //     .upload(password.clone(), token, channel_id)
    //     .export_waterfall_with_password(password.clone());
    //
    // println!("{:?}", serde_json::to_string(&waterfall).unwrap());
    //
    // let mut file = File::create("cool.waterfall").unwrap();
    // file.write_all(serde_json::to_string_pretty(&waterfall).unwrap().as_bytes()).unwrap();
}


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
use crate::downloader::{ByteRangeDownloader, Downloader, FileDownloader, WaterfallDownloader};
use crate::uploader::WaterfallExporter;
use crate::utils::{create_trash_dir};


fn main() {
    create_trash_dir();
    let token = String::from("");
    let channel_id: u64 = 0;

    let password = "password".to_string();

    let downloader = FileDownloader::from_waterfall(Waterfall::from_file("inoxtag.waterfall".to_string()));


    let mut f = File::create("inoxtag.mov").unwrap();
    let range = [20000, 9_000_000];

    let mut r = downloader.get_range(range[0], range[1]);

    let mut to_read = range[1] - range[0];

    while to_read > 0 {
        let mut buffer = vec![0; 2048];
        let read = r.read(&mut buffer).unwrap();
        println!("read: {}, to_read {}", read, to_read);

        f.write_all(&buffer[..read]).unwrap();
        to_read -= read as u64;
    }



    //downloader.download_file("inoxtag.mov".to_string());

    // let waterfall = FileUploader::new_with_threads_count("trash/IMG_0577.mov".to_string(), 24 * 1024 * 1024, 1)
    //     .upload(password.clone(), token, channel_id)
    //     .export_waterfall_with_password(password.clone());
    //
    // waterfall.write_to_file("inoxtag.waterfall".to_string());
}


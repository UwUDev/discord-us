mod utils;
mod uploader;
mod database;
mod downloader;

use crate::database::{create_db, export_waterfall, import_waterfall};
use crate::downloader::safe_download;
use crate::uploader::safe_upload;
use crate::utils::{create_trash_dir, empty_trash};
use crate::utils::Subscription::{Boost, Free};


fn main() {
    create_trash_dir();
    let token = String::from("no.");
    let channel_id = 1146787754915676260u64;

    //create_db("123456");
    //safe_download(2, "123456", "."); // this is a test file but if you want free pfp's you can use this
    let saved_id = safe_upload("123456","Cargo.toml", token, channel_id, Boost);
    export_waterfall(saved_id, "cool.waterfall");
    //import_waterfall("cool.waterfall");
    empty_trash();
}


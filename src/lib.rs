pub mod uploader;
pub mod downloader;
mod http_client;
pub mod common;
pub mod signal;

pub mod pack;
pub mod fs;
pub mod utils;

/// Trait for getting size of something
pub trait Size {
    fn get_size(&self) -> u64;
}
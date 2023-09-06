use std::fs::File;
use std::io::{Write};
use serde::{Deserialize, Serialize};
use hex_buffer_serde::{Hex as _, HexForm};

pub trait FileWritable {
    fn write_to_file(&self, file_path: String);
}

pub trait FileReadable {
    fn from_file(file_path: String) -> Self;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Waterfall {
    pub filename: String,
    pub password: String,
    pub size: u64,

    pub containers: Vec<Container>,
}

impl FileWritable for Waterfall {
    fn write_to_file(&self, file_path: String) {
        let mut file = File::create(file_path).unwrap();
        file.write_all(serde_json::to_string_pretty(&self).unwrap().as_bytes()).unwrap();
    }
}

impl FileReadable for Waterfall {
    fn from_file(file_path: String) -> Self {
        let mut file = File::open(file_path).unwrap();

        serde_json::from_reader(&mut file).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Container {
    pub storage_url: String,
    pub chunk_size: u64,
    pub chunk_count: u64,

    #[serde(with = "HexForm")]
    pub salt: [u8; 16],

    pub bytes_range: [u64; 2],
}

pub enum Subscription {
    Free,
    Basic,
    Classic,
    // yeah you can still buy it
    Boost,
}

impl Subscription {
    pub fn get_max_chunk_upload_size(&self) -> usize {
        match *self {
            Self::Free => 25 * 1024 * 1024,
            Self::Basic => 50 * 1024 * 1024,
            Self::Classic => 100 * 1024 * 1024,
            Self::Boost => 500 * 1024 * 1024,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ResumableFileUpload {
    pub(crate) file_path: String,
    pub(crate) file_size: u64,

    pub(crate) container_size: u32,
    pub(crate) remaining_indexes: Vec<u32>,
    pub(crate) containers: Vec<Container>,

    #[serde(with = "HexForm")]
    pub(crate) file_hash: [u8; 32],

    pub(crate) thread_count: usize,
}

impl FileWritable for ResumableFileUpload {
    fn write_to_file(&self, file_path: String) {
        let mut file = File::create(file_path).unwrap();
        file.write_all(serde_json::to_string_pretty(&self).unwrap().as_bytes()).unwrap();
    }
}

impl FileReadable for ResumableFileUpload {
    fn from_file(file_path: String) -> Self {
        let mut file = File::open(file_path).unwrap();

        serde_json::from_reader(&mut file).unwrap()
    }
}
use std::fs::File;
use std::io::Write;
use serde::{Deserialize, Serialize};

pub trait FileWritable {
    fn write_to_file(&self, file_path: String);
}

#[derive(Serialize, Deserialize, Clone)]
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

impl Waterfall {
    pub fn from_file(file_path: String) -> Self {
        serde_json::from_str::<Waterfall>(
            &std::fs::read_to_string(file_path).unwrap()
        ).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Container {
    pub storage_url: String,
    pub chunk_size: u64,
    pub chunk_count: u64,

    pub salt: [u8; 16],

    pub bytes_range: [u64; 2],
}

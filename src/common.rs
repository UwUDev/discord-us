use std::fs::File;
use std::io::Write;
use serde::{Deserialize, Serialize};
use crate::downloader::{FileDownloader, WaterfallDownloader};

#[derive(Serialize, Deserialize, Clone)]
pub struct Waterfall {
    pub filename: String,
    pub password: String,
    pub size: u64,

    pub containers: Vec<Container>,
}

impl Waterfall {
    pub fn download(&self, password: String) {
        let mut file = File::create(self.filename.clone()).unwrap();

        for container in self.containers.clone() {
            let mut downloader = FileDownloader::from_waterfall(self.clone());
            downloader.set_password(password.clone());

            let chunk_count = container.chunk_count;

            let download_container = downloader.get_container_downloader(container);
            let chunks: Vec<Vec<u8>> = download_container.get_chunks(0, chunk_count as usize).unwrap();

            for chunk in chunks {
                file.write_all(&chunk).unwrap();
            }
            file.flush().unwrap();
        }

        file.flush().unwrap();
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

use std::cmp::min;
use std::fs::File;
use std::io::{Read, Write};
use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use reqwest::blocking::Client;
use reqwest::blocking::{Response};
use reqwest::{StatusCode};
use sha2::{Digest, Sha256};
use crate::common::{Container, Waterfall};
use crate::http_client::create_client;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

const METADATA_SIZE: usize = 64;

pub trait Downloader {
    fn download_file(&self, file_path: String);
}

pub trait WaterfallDownloader {
    fn from_waterfall(waterfall: Waterfall) -> Self;
}

pub trait ByteRangeDownloader {
    fn get_size(&self) -> u64;

    fn get_range(&self, start: u64, end: u64) -> Result<Vec<u8>, &str>;
}

pub struct FileDownloader {
    waterfall: Waterfall,
    password: String,
}

impl WaterfallDownloader for FileDownloader {
    fn from_waterfall(waterfall: Waterfall) -> Self {
        let waterfall = waterfall.clone();
        let password = waterfall.password.clone();

        FileDownloader {
            waterfall,
            password,
        }
    }
}

impl FileDownloader {
    pub fn set_password(&mut self, password: String) -> &mut FileDownloader {
        self.password = password.clone();

        self
    }

    pub fn get_container_downloader(&self, container: Container) -> ContainerDownloader {
        ContainerDownloader::new(container.clone(), self.waterfall.size, self.password.clone())
    }
}

#[derive(Clone)]
pub struct ContainerDownloader {
    container: Container,
    key: [u8; 32],
    client: Client,
    file_size: u64,
}

impl ContainerDownloader {
    fn hash_key(encryption_password: String, salt: [u8; 16]) -> [u8; 32] {
        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(encryption_password.as_bytes(), &salt, 10000, &mut key);
        key
    }

    pub fn new(container: Container, file_size: u64, encryption_password: String) -> Self {
        let key = Self::hash_key(encryption_password, container.salt);

        ContainerDownloader {
            container,
            key,
            file_size,
            client: create_client(),
        }
    }

    pub fn get_byte_stream(&self, chunk_offset: u64, count: usize) -> Result<ByteStream, &str> {
        ByteStream::new(self.container.clone(), self.key.clone(), self.file_size, chunk_offset, count)
    }

    pub fn get_chunks(&self, chunk_offset: u64, count: usize) -> Result<Vec<Vec<u8>>, &str> {
        let mut chunks: Vec<Vec<u8>> = Vec::with_capacity(count);

        let mut downloader = self.get_byte_stream(chunk_offset, count).unwrap();

        for _i in 0..count {
            let mut chunk: Vec<u8> = vec![0; self.container.chunk_size as usize - METADATA_SIZE];

            downloader.read(&mut chunk).expect("TODO: panic message");

            chunks.push(chunk);
        }

        Ok(chunks.clone())
    }
}

pub struct ByteStream {
    container: Container,
    key: [u8; 32],
    file_size: u64,

    chunk_offset: u64,
    count: usize,

    current_chunk: u64,
    buffer: Vec<u8>,
    buffer_cursor: usize,

    response: Response,
}

impl ByteStream {
    pub fn new(container: Container, key: [u8; 32], file_size: u64, chunk_offset: u64, count: usize) -> Result<Self, &'static str> {
        let range_start = chunk_offset * container.chunk_size;
        let range_stop = range_start + (count as u64 * container.chunk_size);

        let mut response = create_client().get(container.storage_url.clone())
            .header("User-Agent", "Mozilla/5.0")
            .header("Range", format!("bytes={}-{}", range_start, range_stop))
            .send()
            .unwrap();

        if response.status() != StatusCode::from_u16(206).unwrap() {
            return Err("Invalid response status");
        }

        let chunk_size = container.chunk_size;

        Ok(Self { container, key, file_size, chunk_offset, count, current_chunk: 0, buffer: vec![0; chunk_size as usize], buffer_cursor: chunk_size as usize, response })
    }

    fn download_chunk(&mut self) -> Result<(), &str> {
        let mut buffer = vec![0; self.container.chunk_size as usize];

        let mut read = 0;

        while read < buffer.len() {
            let r = self.response.read(&mut buffer[read..]).expect("TODO: panic message");
            read += r;
        }

       // println!("Download: Read {} bytes (chunk {})", read, self.current_chunk + self.chunk_offset);

        let chunk_start = self.container.bytes_range[0] + (self.current_chunk + self.chunk_offset) * (self.container.chunk_size - METADATA_SIZE as u64);

        let chunk_stop = min(self.file_size, chunk_start + self.container.chunk_size - (METADATA_SIZE as u64));

        return match self.decrypt_and_verify_chunk(&mut buffer, (chunk_stop - chunk_start) as usize) {
            Ok(data) => {
                self.buffer = data;
                Ok(())
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                Err("Cannot decrypt chunk")
            }
        };
    }

    fn decrypt_and_verify_chunk(&self, chunk: &mut Vec<u8>, content_size: usize) -> Result<Vec<u8>, &str> {
        //println!("Decrypting and verifying chunk of size {} (real {})", content_size, chunk.len());

        let chunk_size = self.container.chunk_size as usize;

        if chunk.len() != chunk_size {
            return Err("Chunk size mismatch");
        }

        let salt = chunk[(chunk_size - 48)..(chunk_size - 32)].to_vec();
        let hash = chunk[(chunk_size - 32)..].to_vec();

        //println!("Read salt: {:X?}", salt);
        //println!("Read hash: {:X?}", hash);

        let cipher = Aes256Cbc::new_from_slices(
            &self.key.clone(),
            &salt.clone(),
        ).unwrap();

        if let Err(err) = cipher.decrypt(&mut chunk[0..(chunk_size - 48)]) {
            eprintln!("Error: {err}");
            return Err("Cannot decrypt chunk");
        }

        // compute hash
        let data = chunk[0..content_size].to_vec();

        let mut hasher = Sha256::new();
        hasher.update(&chunk[0..((self.container.chunk_size as usize) - (METADATA_SIZE))]);
        let data_hash = hasher.finalize();

        if hash != data_hash.to_vec() {
            return Err("Hash mismatch");
        }

        Ok(data.to_vec())
    }
}

impl Read for ByteStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        //println!("Read : size({:?}) | residual buffer_cursor {:?}", buf.len(), self.buffer_cursor);

        let mut read = 0;

        while read < buf.len() {
           // println!("Read loop {:?} over {:?}", self.buffer_cursor, self.buffer.len());

            if self.buffer_cursor < self.buffer.len() {
                let remain = min(buf.len() - read, self.buffer.len() - self.buffer_cursor);
                buf[read..(read + remain)].clone_from_slice(&self.buffer[self.buffer_cursor..(self.buffer_cursor + remain)]);
                read += remain;
                self.buffer_cursor += remain;
            }

            if self.buffer_cursor >= self.buffer.len() {
               // println!("Count: {:?} | Current chunk: {:?}", self.count, self.current_chunk);

                if self.current_chunk >= self.count as u64 {
                    return Ok(read);
                } else {
                    self.download_chunk().expect("TODO: panic message");
                    self.current_chunk += 1;
                    self.buffer_cursor = 0;
                }
            }
        }

        Ok(read)
    }
}

impl Downloader for FileDownloader {
    fn download_file(&self, file_path: String) {
        let mut f = File::create(file_path).unwrap();

        for ctn in self.waterfall.clone().containers.clone() {

            let container = self.get_container_downloader(ctn.clone());
            let mut stream = container.get_byte_stream(0, ctn.chunk_count as usize).unwrap();

            let mut buf = [0u8; 65536 - 64];
            let mut to_write = (ctn.bytes_range[1] - ctn.bytes_range[0]) as usize;

            //println!("to_write: {}", to_write);

            while to_write > 0  {
                let read = stream.read(&mut buf).unwrap();

                let c = to_write.min(read);
                f.write_all(&mut buf[..c]).expect("TODO: panic message");
                // println!("to_write: {}", to_write);
                to_write -= c;
            }

        }
    }
}

impl ByteRangeDownloader for FileDownloader {
    fn get_size(&self) -> u64 {
        self.waterfall.size
    }

    fn get_range(&self, start: u64, end: u64) -> Result<Vec<u8>, &str> {
        // get containers associated with that range
        let mut containers: Vec<Container> = Vec::new();

        containers.sort_by(|a, b| a.bytes_range[0].cmp(&b.bytes_range[0]));

        todo!()
    }
}
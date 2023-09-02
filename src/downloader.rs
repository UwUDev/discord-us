use std::cmp::{max, min};
use std::error::Error;
use std::io::Read;
use std::sync::{Arc, Mutex};
use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use sha256::digest;
use sha2::{Digest, Sha256};
use crate::common::{Container, Waterfall};
use crate::database::{get_blocks, get_file_md5, get_file_name, get_hashed_pass};
use crate::http_client::create_client;
use crate::utils::{Block, calculate_file_md5, decrypt_file, repack_blocks};

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

const METADATA_SIZE: usize = 64;

pub trait Downloader {
    fn download(&self);
}

pub trait WaterfallDownloader {
    fn from_waterfall(waterfall: Waterfall) -> Self;
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

    pub fn get_chunks(&self, chunk_offset: u64, count: usize) -> Result<Vec<Vec<u8>>, &str> {
        let mut chunks: Vec<Vec<u8>> = Vec::with_capacity(count);

        let range_start = chunk_offset * self.container.chunk_size;
        let range_stop = range_start + (count as u64 * self.container.chunk_size);

        let mut response = self.client.get(self.container.storage_url.clone())
            .header("User-Agent", "Mozilla/5.0")
            .header("Range", format!("bytes={}-{}", range_start, range_stop))
            .send()
            .unwrap();

        println!("Download: Got response {:#?}", response);

        if response.status() != StatusCode::from_u16(206).unwrap() {
            return Err("Invalid response status");
        }

        for i in 0..count {
            let mut chunk = vec![0; self.container.chunk_size as usize];

            let mut read = 0;

            while read < chunk.len() {
                let r = response.read(&mut chunk[read..]).expect("TODO: panic message");
                read += r;
            }

            println!("Download: Read {} bytes (chunk {})", read, (chunk_offset as usize) + i);

            let chunk_start = self.container.bytes_range[0] + (chunk_offset + (i as u64)) * (self.container.chunk_size - METADATA_SIZE as u64);

            let chunk_stop = min(self.file_size, chunk_start + self.container.chunk_size - (METADATA_SIZE as u64));

            match self.decrypt_and_verify_chunk(&mut chunk, (chunk_stop - chunk_start) as usize) {
                Ok(data) => {
                    chunks.push(data);
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    return Err("Cannot decrypt chunk");
                }
            }
        }

        Ok(chunks.clone())
    }

    fn decrypt_and_verify_chunk(&self, chunk: &mut Vec<u8>, content_size: usize) -> Result<Vec<u8>, &str> {
        println!("Decrypting and verifying chunk of size {} (real {})", content_size, chunk.len());

        let chunk_size = self.container.chunk_size as usize;

        if chunk.len() != chunk_size {
            return Err("Chunk size mismatch");
        }

        let salt = chunk[(chunk_size - 48)..(chunk_size - 32)].to_vec();
        let hash = chunk[(chunk_size - 32)..].to_vec();

        println!("Read salt: {:X?}", salt);
        println!("Read hash: {:X?}", hash);

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


pub fn safe_download(file_id: usize, pass: &str, output_dir: &str) {
    let pass_digest = digest(pass.as_bytes());
    let hashed_pass = get_hashed_pass(file_id);

    if pass_digest != hashed_pass {
        panic!("Wrong password");
    }

    let output_file = format!("{}/{}", output_dir, get_file_name(file_id));
    let mut blocks = get_blocks(file_id);

    println!("Downloading {} blocks", blocks.len());
    download_blocks(&mut blocks);
    println!("Downloaded {} blocks", blocks.len());

    println!("Repacking blocks");
    let enc_file_path = repack_blocks(blocks);
    println!("Repacked blocks");

    println!("Decrypting file");
    decrypt_file(enc_file_path.as_str(), output_file.clone().as_str(), pass);
    println!("Decrypted file");

    let md5 = get_file_md5(file_id);

    println!("Verifying MD5");
    let final_md5 = calculate_file_md5(output_file.as_str()).unwrap();

    if md5 != final_md5 {
        panic!("MD5 mismatch");
    }

    println!("All done!");
}

fn download_blocks(blocks: &mut Vec<Block>) {
    let client = reqwest::blocking::Client::builder()
        .brotli(true)
        .gzip(true)
        .build()
        .unwrap();

    let mut blk_num = 0;
    let block_count = blocks.len();

    for block in blocks {
        let url = block.url.clone().unwrap();
        let path = url.split("/").last().unwrap();
        let filename = "trash/".to_owned() + path.split("/").last().unwrap();

        let mut res = client.get(url.as_str())
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .unwrap();

        let mut file = std::fs::File::create(filename.clone()).unwrap();
        std::io::copy(&mut res, &mut file).unwrap();
        blk_num += 1;

        let digest = sha256::digest_file(filename.as_str()).unwrap();

        println!("Downloaded block {}/{} ({} bytes) [{}]", blk_num, block_count, block.size, digest);

        if digest != block.hash {
            panic!("Digest mismatch");
        }

        block.path = filename.to_string();
    }
}
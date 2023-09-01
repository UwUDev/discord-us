use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use aes::{Aes256, NewBlockCipher};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use reqwest::blocking::{Body, Client};
use serde_json::json;
use sha256::digest;
use sha2::{Digest, Sha256};
use crate::database::save_upload;
use crate::utils::{Block, calculate_file_md5, empty_trash, encrypt_file, Subscription, to_blocks};
use threadpool::ThreadPool;
use rand::{RngCore, thread_rng};
use crate::http_client::{create_client, prepare_discord_request};

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

pub trait Uploader {
    fn upload(&mut self, encryption_password: String, token: String, channel_id: u64);
}

const CHUNK_SIZE: u32 = 1 << 23;

const METADATA_SIZE: usize = 64;

pub struct FileUploader {
    file_path: String,
    file_size: u64,

    threads_count: u32,
    container_size: u32,

    current_container_index: Arc<Mutex<u32>>,
}

impl FileUploader {
    pub fn new(file_path: String, container_size: u32) -> FileUploader {
        FileUploader::new_with_threads_count(file_path, container_size, 2)
    }

    pub fn new_with_threads_count(file_path: String, container_size: u32, threads_count: u32) -> FileUploader {
        let file_size = std::fs::metadata(file_path.clone()).unwrap().len();

        FileUploader {
            file_size,
            file_path: file_path.clone(),
            container_size,
            threads_count,
            current_container_index: Arc::new(Mutex::new(0)),
        }
    }

    fn chunks_per_container(&self) -> u32 {
        self.container_size / CHUNK_SIZE
    }

    fn container_count(&self) -> u32 {
        let chunk_count = (self.file_size / (CHUNK_SIZE as u64)) + 1;

        let chunks_per_container = self.chunks_per_container();

        println!("?: {:?}, {:?}", chunk_count, chunks_per_container);

        (chunk_count as f64 / chunks_per_container as f64).ceil() as u32
    }
}

impl Uploader for FileUploader {
    fn upload(&mut self, encryption_password: String, token: String, channel_id: u64) {
        let thread_count = self.threads_count.clone();

        let container_count = self.container_count();

        println!("Container count: {:?}", container_count);

        let pool = ThreadPool::new(thread_count as usize);

        for i in 0..thread_count {
            // create file uploader
            let mut uploader = FileThreadedUploader::new(
                container_count,
                self.current_container_index.clone(),
                self.file_path.clone(),
                self.container_size,
                token.clone(),
                channel_id,
                encryption_password.clone(),
                self.file_size,
            );

            pool.execute(move || {
                uploader.start_uploading();
            });
        }

        pool.join();
    }
}

struct FileThreadedUploader {
    container_count: u32,
    current_container_index: Arc<Mutex<u32>>,

    file_path: String,
    file_size: u64,
    container_size: u32,
    token: String,
    channel_id: u64,
    encryption_password: String,

    client: Client,
}


impl FileThreadedUploader {
    fn new(container_count: u32, current_container_index: Arc<Mutex<u32>>, file_path: String, container_size: u32, token: String, channel_id: u64, encryption_password: String, file_size: u64) -> FileThreadedUploader {
        FileThreadedUploader {
            container_count,
            container_size,
            file_path,
            current_container_index,
            token,
            channel_id,
            encryption_password,
            file_size,
            client: create_client(),
        }
    }

    fn start_uploading(&mut self) {
        let mut container_index = self.get_processing_container_index();

        while container_index != -1 {
            //self.upload_container(container_index);
            println!("Uploading Container {:?}", container_index);

            self.upload(container_index as u32);

            container_index = self.get_processing_container_index();
        }

        return;
    }

    fn upload(&mut self, container_index: u32) {
        let mut salt = [0u8; 16];

        thread_rng().fill_bytes(&mut salt);

        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(self.encryption_password.as_bytes(), &salt, 10000, &mut key);

        let mut file = File::open(self.file_path.clone()).unwrap();

        file.seek(SeekFrom::Current(
            (container_index * self.container_size) as i64
                - ((METADATA_SIZE as i64) * ((container_index as i64) - 1) * (self.chunks_per_container() as i64))
        )).unwrap();

        let mut remaining_size = min(self.container_size as u64, (self.file_size - (container_index * self.container_size) as u64));


        let (upload_url, upload_filename) = self.request_attachment(remaining_size);

        let body = Body::new();

        // self.client.put(upload_url)
        //     .header("accept-encoding", "gzip")
        //     .header("connection", "Keep-Alive")
        //     .header("content-length", remaining_size)
        //     .header("content-type", "application/x-x509-ca-cert")
        //     .header("host", "discord-attachments-uploads-prd.storage.googleapis.com")
        //     .header("user-agent", "Discord-Android/192013;RNA")
        //     .body(file);


        let mut bytes_processed = 0;

        while bytes_processed < self.container_size {}
    }

    fn get_processing_container_index(&mut self) -> i32 {
        let mut value = self.current_container_index.lock().unwrap();

        if *value + 1 > self.container_count {
            return -1;
        }

        *value += 1;

        return value.clone() as i32;
    }

    fn chunks_per_container(&self) -> u32 {
        self.container_size / CHUNK_SIZE
    }

    fn request_attachment(&self, size: u64) -> (String, String) {
        let url = format!("https://discord.com/api/v9/channels/{}/attachments", self.channel_id);

        let payload = json!(
            {
                "files": [
                    {
                        "filename": "data.enc",
                        "file_size": size,
                        "id": "8"
                    }
                ]
            }
        );


        let mut request = self.client.post(url);

        request = prepare_discord_request(request, self.token.clone());

        let resp = request.json(&payload).send().unwrap().json::<serde_json::Value>().unwrap();

        let upload_url = resp["attachments"][0]["upload_url"].as_str().unwrap();
        let upload_filename = resp["attachments"][0]["upload_filename"].as_str().unwrap();

        return (upload_url.to_string(), upload_filename.to_string());
    }
}


struct CustomBody<'a> {
    salt: [u8; 16],
    key: [u8; 32],

    remaining_size: u64,
    file: &'a mut File,

    current_salt: [u8; 32],
    buffer_cursor: usize,
    buffer: [u8; CHUNK_SIZE as usize],
}

impl CustomBody<'_> {
    fn do_one_chunk(&mut self) {
        let content_size = (CHUNK_SIZE as usize) - METADATA_SIZE;

        let bytes_read = self.file.read(&mut self.buffer[..content_size]).unwrap();

        // compute hash
        let mut hasher = Sha256::new();
        hasher.update(&self.buffer[..content_size]);
        let hash = hasher.finalize();


        // encrypt data
        let cipher = Aes256Cbc::new_from_slices(
            &self.key.clone(),
            &self.current_salt.clone(),
        ).unwrap();

        let mut encrypted = cipher.encrypt(&mut self.buffer[..content_size], content_size).unwrap();

        // add at end the iv
        encrypted[..content_size].clone_from_slice(&self.current_salt.clone());

        encrypted[..(content_size + self.current_salt.len())].clone_from_slice(&self.current_salt.clone());

        self.remaining_size -= CHUNK_SIZE as u64;
    }

    pub fn new(salt: [u8; 16], key: [u8; 32], remaining_size: u64, file: &mut File, current_salt: [u8; 32]) -> Self {
        Self { salt, key, remaining_size, file, current_salt, buffer: [0u8; CHUNK_SIZE as usize], buffer_cursor: 0 }
    }
}

impl Read for CustomBody<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read = 0;

        while read < buf.len() {
            if self.remaining_size <= 0 {
                return Ok(read);
            }


            if self.buffer_cursor < CHUNK_SIZE as usize {

            } else {
                self.do_one_chunk();
                self.buffer_cursor = 0;
            }

            let remaining = buf.len() - read;

            let to_read = min(remaining, CHUNK_SIZE as usize);

            buf[read..(read + to_read)].clone_from_slice(&self.buffer[..to_read]);

            read += to_read;
            self.buffer_cursor += to_read;
        }

        Ok(read)
    }
}


pub fn safe_upload(pass: &str, input_file: &str, token: String, channel_id: u64, sub: Subscription) -> usize {
    let uuid = uuid::Uuid::new_v4();
    let file_size = std::fs::metadata(input_file).unwrap().len();

    println!("Calculating MD5");
    let md5 = calculate_file_md5(input_file).unwrap();
    println!("md5: {}", md5);

    let enc_file_path = format!("trash/{}.enc", uuid);

    println!("Encrypting file");
    encrypt_file(input_file, enc_file_path.clone().as_str(), pass);
    println!("Encrypted file");

    println!("Splitting file into blocks");
    let mut blocks = to_blocks(enc_file_path.clone().as_str(), sub);
    println!("Split file into blocks");

    println!("Uploading blocks");
    upload_blocks(
        &mut blocks,
        token,
        channel_id,
    );
    println!("Uploaded blocks");


    empty_trash();

    let hashed_pass = digest(pass.as_bytes());

    let block_count = blocks.len();

    let input_file_name = input_file.split("/").last().unwrap();

    println!("Saving upload");
    let saved_id = save_upload(
        input_file_name,
        file_size,
        md5.as_str(),
        hashed_pass.as_str(),
        block_count,
        &blocks,
    );

    println!("All done!");

    saved_id
}

pub fn upload_blocks(blocks: &mut Vec<Block>, token: String, channel_id: u64) {
    let client = Client::builder()
        .timeout(Duration::from_secs(60 * 60))
        .brotli(true)
        .gzip(true)
        .build()
        .unwrap();

    let mut blk_num = 0;
    let mut block_count = blocks.len();
    for block in blocks.iter_mut() {
        blk_num += 1;

        print!("Uploading block {}/{} ({} bytes) [{}]", blk_num, block_count, block.size, block.hash);
        std::io::stdout().flush().unwrap();

        let url = format!("https://discord.com/api/v9/channels/{}/attachments", channel_id);

        let path = block.path.clone();
        let filename = path.split("/").last().unwrap();
        let payload = json!(
            {
                "files": [
                    {
                        "filename": filename,
                        "file_size": block.size,
                        "id": "8"
                    }
                ]
            }
        );


        let resp = client.post(url)
            .header("Authorization", token.clone())
            .header("Content-Type", "application/json")
            .header("X-Super-Properties", "eyJvcyI6IkFuZHJvaWQiLCJicm93c2VyIjoiRGlzY29yZCBBbmRyb2lkIiwiZGV2aWNlIjoiYmx1ZWpheSIsInN5c3RlbV9sb2NhbGUiOiJmci1GUiIsImNsaWVudF92ZXJzaW9uIjoiMTkyLjEzIC0gcm4iLCJyZWxlYXNlX2NoYW5uZWwiOiJnb29nbGVSZWxlYXNlIiwiZGV2aWNlX3ZlbmRvcl9pZCI6IjhkZGU4M2IzLTUzOGEtNDJkMi04MzExLTM1YmFlY2M2YmJiOCIsImJyb3dzZXJfdXNlcl9hZ2VudCI6IiIsImJyb3dzZXJfdmVyc2lvbiI6IiIsIm9zX3ZlcnNpb24iOiIzMyIsImNsaWVudF9idWlsZF9udW1iZXIiOjE5MjAxMzAwMTEzNzczLCJjbGllbnRfZXZlbnRfc291cmNlIjpudWxsLCJkZXNpZ25faWQiOjB9")
            .header("Accept-Language", "fr-FR")
            .header("X-Discord-Locale", "fr")
            .header("X-Discord-Timezone", "Europe/Paris")
            .header("X-Debug-Options", "bugReporterEnabled")
            .header("User-Agent", "Discord-Android/192013;RNA")
            .header("Host", "discord.com")
            .header("Connection", "Keep-Alive")
            .header("Accept-Encoding", "gzip")
            .json(&payload)
            .send().unwrap().json::<serde_json::Value>().unwrap();

        let upload_url = resp["attachments"][0]["upload_url"].as_str().unwrap();
        let upload_filename = resp["attachments"][0]["upload_filename"].as_str().unwrap();

        let file = File::open(&block.path).unwrap();

        client.put(upload_url)
            .header("accept-encoding", "gzip")
            .header("connection", "Keep-Alive")
            .header("content-length", block.size)
            .header("content-type", "application/x-x509-ca-cert")
            .header("host", "discord-attachments-uploads-prd.storage.googleapis.com")
            .header("user-agent", "Discord-Android/192013;RNA")
            .body(file)
            .send().unwrap();


        let url = format!("https://discord.com/api/v9/channels/{}/messages", channel_id);

        let payload = json!(
            {
                "content": "",
                "channel_id": channel_id,
                "type": 0,
                "attachments": [
                    {
                        "id": "0",
                        "filename": filename,
                        "uploaded_filename": upload_filename
                    }
                ]
            }
        );

        let resp = client.post(url)
            .header("Authorization", token.clone())
            .header("X-Super-Properties", "eyJvcyI6IkFuZHJvaWQiLCJicm93c2VyIjoiRGlzY29yZCBBbmRyb2lkIiwiZGV2aWNlIjoiYmx1ZWpheSIsInN5c3RlbV9sb2NhbGUiOiJmci1GUiIsImNsaWVudF92ZXJzaW9uIjoiMTkyLjEzIC0gcm4iLCJyZWxlYXNlX2NoYW5uZWwiOiJnb29nbGVSZWxlYXNlIiwiZGV2aWNlX3ZlbmRvcl9pZCI6IjhkZGU4M2IzLTUzOGEtNDJkMi04MzExLTM1YmFlY2M2YmJiOCIsImJyb3dzZXJfdXNlcl9hZ2VudCI6IiIsImJyb3dzZXJfdmVyc2lvbiI6IiIsIm9zX3ZlcnNpb24iOiIzMyIsImNsaWVudF9idWlsZF9udW1iZXIiOjE5MjAxMzAwMTEzNzczLCJjbGllbnRfZXZlbnRfc291cmNlIjpudWxsLCJkZXNpZ25faWQiOjB9")
            .header("Accept-Language", "fr-FR")
            .header("X-Discord-Locale", "fr")
            .header("X-Discord-Timezone", "Europe/Paris")
            .header("X-Debug-Options", "bugReporterEnabled")
            .header("User-Agent", "Discord-Android/192013;RNA")
            .header("Content-Type", "application/json")
            .header("Host", "discord.com")
            .header("Connection", "Keep-Alive")
            .header("Accept-Encoding", "gzip")
            .json(&payload)
            .send().unwrap().json::<serde_json::Value>().unwrap();

        let file_url = resp["attachments"][0]["url"].as_str().unwrap();

        block.url = Some(file_url.to_string());

        print!("\rUploaded block {}/{} ({} bytes) [{}]", blk_num, block_count, block.size, block.hash);
    }
}
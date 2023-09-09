use std::cmp::{min};
use std::collections::VecDeque;
use std::marker::Send;
use std::fs::{File, metadata};
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use aes::{Aes256};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use reqwest::blocking::{Body, Client};
use serde_json::json;
use sha2::{Digest, Sha256};
use threadpool::ThreadPool;
use rand::{RngCore, thread_rng};
use crate::common::{Container, Waterfall, FileReadable, FileWritable, ResumableFileUpload};
use crate::http_client::{create_client, prepare_discord_request};
use crate::signal::{LinearPartSignal, PartProgression, ProgressionRange, ReportSignal};

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

pub trait Uploader<T, R>
    where T: Sized + Clone
{
    fn upload(&mut self, data: T) -> R;
}

pub trait WaterfallExporter {
    fn export_waterfall(&self) -> Waterfall {
        self.export_waterfall_with_password(String::new())
    }
    fn export_waterfall_with_password(&self, password: String) -> Waterfall;
}

impl WaterfallExporter for ResumableFileUpload {
    fn export_waterfall_with_password(&self, password: String) -> Waterfall {
        Waterfall {
            containers: self.containers.clone(),
            password: password.clone(),
            filename: self.file_path.clone(),
            size: self.file_size,
        }
    }
}

pub trait ResumableUploader<T>
    where T: FileWritable + FileReadable + Clone {
    fn export_resume_session(&self) -> T;

    fn from_resume_session(resume_session: T) -> std::io::Result<Self>
        where Self: Sized;
}

const CHUNK_SIZE: u32 = 1 << 16;

const METADATA_SIZE: usize = 64;

#[derive(Clone)]
pub struct FileUploader {
    file_path: String,
    file_size: u64,

    container_size: u32,

    remaining_container_indexes: Arc<Mutex<VecDeque<u32>>>,
    current_downloading_indexes: Arc<Mutex<Vec<u32>>>,
    containers: Arc<Mutex<Vec<Container>>>,

    pool: Arc<ThreadPool>,

    running: Arc<RwLock<bool>>,
}

impl FileUploader {
    pub fn new(file_path: String, container_size: u32) -> FileUploader {
        FileUploader::new_with_threads_count(file_path, container_size, 2)
    }

    pub fn new_with_threads_count(file_path: String, container_size: u32, threads_count: u32) -> FileUploader {
        let file_size = Self::file_size(file_path.clone());

        let container_count = Self::container_count(file_size, container_size as u64);
        let mut deque: VecDeque<u32> = VecDeque::with_capacity(container_count);

        for i in 0..container_count {
            deque.push_back(i as u32 + 1);
        }

        FileUploader {
            file_size,
            file_path: file_path.clone(),
            container_size,
            remaining_container_indexes: Arc::new(Mutex::new(deque)),
            containers: Arc::new(Mutex::new(Vec::new())),
            current_downloading_indexes: Arc::new(Mutex::new(Vec::new())),
            pool: Arc::new(ThreadPool::new(threads_count as usize)),

            running: Default::default(),
        }
    }

    fn file_size(file_path: String) -> u64 {
        let meta = metadata(file_path).unwrap();

        meta.len()
    }

    fn container_count(file_size: u64, container_size: u64) -> usize {
        let chunk_count = (file_size / (CHUNK_SIZE as u64 - METADATA_SIZE as u64)) + 1;

        let chunks_per_container = container_size / (CHUNK_SIZE as u64);

        (chunk_count as f64 / chunks_per_container as f64).ceil() as usize
    }

    fn file_hash(file_path: String) -> [u8; 32] {
        let mut hasher = Sha256::new();
        // hash file
        let mut file = File::open(file_path).unwrap();
        let mut buffer = [0u8; 1 << 16];
        loop {
            let bytes_read = file.read(&mut buffer).unwrap();
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[0..bytes_read]);
        }
        hasher.finalize().into()
    }

    fn compute_chunk_count(&self) -> usize {
        let real_size = CHUNK_SIZE as usize - METADATA_SIZE;

        let chunks_per_container = (self.container_size as usize) / (CHUNK_SIZE as usize);

        let container = Self::container_count(self.file_size, self.container_size as u64);

        // the first N-1 containers are always full:
        let mut chunk_count = (container - 1) * chunks_per_container;

        // for the last container, we need to compute the remaining bytes.
        let used_bytes = chunk_count * (real_size);

        let remaining = self.file_size as usize - used_bytes;

        // we ask ourself how much containers can fit in remaing space
        chunk_count += (remaining / real_size) + 1;

        chunk_count
    }

    pub fn get_uploaded_ranges(&self) -> Vec<ProgressionRange<u64>> {
        self.containers.lock().unwrap().iter().map(|container| {
            let chunk_count_before = container.bytes_range[0] / (CHUNK_SIZE as u64 - METADATA_SIZE as u64);

            let c_start = container.bytes_range[0] + (chunk_count_before * METADATA_SIZE as u64);

            let chunk_count = (container.bytes_range[1] - container.bytes_range[0]) / (CHUNK_SIZE as u64 - METADATA_SIZE as u64);
            let mut c_end = container.bytes_range[1] + chunk_count * METADATA_SIZE as u64;

            if c_end % (CHUNK_SIZE as u64) > 0 {
                c_end += (CHUNK_SIZE as u64) - c_end % (CHUNK_SIZE as u64); // add extra padding
            }

            ProgressionRange::of(c_start, c_end)
        }).collect()
    }

    pub fn get_total_upload_size(&self) -> u64 {
        self.compute_chunk_count() as u64 * CHUNK_SIZE as u64
    }

    pub fn get_thread_pool(&self) -> Arc<ThreadPool> {
        self.pool.clone()
    }

    pub fn set_running(&self, running: bool) {
        let mut lock = self.running.write().unwrap();

        *lock = running;
    }

    pub fn get_running_state(&self) -> Arc<RwLock<bool>> {
        self.running.clone()
    }
}

#[derive(Clone)]
pub struct FileUploadArguments {
    encryption_password: String,
    token: String,
    channel_id: u64,

    signal: Option<Box<dyn ReportSignal<ProgressionRange<u64>>>>,
    join: bool,
}

// impl Clone for Box<dyn ReportSignal<ProgressionRange<u64>>> {
//     fn clone(&self) -> Self {
//
//     }
// }

impl FileUploadArguments {
    pub fn new(encryption_password: String, token: String, channel_id: u64) -> FileUploadArguments {
        FileUploadArguments {
            encryption_password,
            token,
            channel_id,
            signal: None,
            join: true,
        }
    }

    pub fn with_signal(&mut self, signal: &PartProgression<u64>) -> &Self {
        self.signal = Some(Box::new(signal.clone()));

        self.join = false;

        self
    }
}


impl Uploader<FileUploadArguments, u64> for FileUploader {
    /// Upload the file using the arguments
    /// Returning the number of bytes uploaded
    /// (Or being uploaded if a signal is passed)
    fn upload(&mut self, arguments: FileUploadArguments) -> u64 {
        // if we come from a resume session, we can already populate the signal
        if let Some(mut signal) = arguments.signal.clone() {
            for range in self.get_uploaded_ranges().iter() {
                signal.report_data(range.clone());
            }
        }

        {
            let mut lock = self.running.write().unwrap();

            *lock = true;
        }

        for _ in 0..self.pool.max_count() {
            // create file uploader
            let mut uploader = FileThreadedUploader::new(
                self.remaining_container_indexes.clone(),
                self.file_path.clone(),
                self.container_size,
                arguments.clone(),
                self.file_size,
                self.containers.clone(),
                self.current_downloading_indexes.clone(),
                self.running.clone(),
            );

            self.pool.execute(move || {
                uploader.start_uploading();
            });
        }

        if arguments.join {
            self.pool.join();
        }

        self.compute_chunk_count() as u64 * CHUNK_SIZE as u64
    }
}

impl WaterfallExporter for FileUploader {
    fn export_waterfall_with_password(&self, password: String) -> Waterfall {
        let containers = self.containers.lock().unwrap().clone();

        Waterfall {
            containers,
            size: self.file_size,
            filename: self.file_path.clone(),
            password: password.clone(),
        }
    }
}

impl ResumableUploader<ResumableFileUpload> for FileUploader {
    fn export_resume_session(&self) -> ResumableFileUpload {
        // Collect remaining indexes
        let remaining_container_indexes = self.remaining_container_indexes.lock().unwrap().clone();

        // Collect containers
        let containers = self.containers.lock().unwrap().clone();

        // collect working indexes
        let working_indexes = self.current_downloading_indexes.lock().unwrap().clone();

        // construct file hash
        let file_hash = Self::file_hash(self.file_path.clone());

        // push all remaining indexes
        let mut remaining_indexes = Vec::with_capacity(remaining_container_indexes.len() + working_indexes.len());

        for index in remaining_container_indexes {
            remaining_indexes.push(index);
        }

        for index in working_indexes {
            remaining_indexes.push(index);
        }

        ResumableFileUpload {
            file_path: self.file_path.clone(),
            file_size: self.file_size,
            container_size: self.container_size,
            remaining_indexes,
            containers,
            file_hash,
            thread_count: self.pool.max_count(),
        }
    }

    fn from_resume_session(resume_session: ResumableFileUpload) -> std::io::Result<Self>
        where Self: Sized {
        let file_size = Self::file_size(resume_session.file_path.clone());

        if file_size != resume_session.file_size {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "File size mismatch"));
        }

        let file_hash = Self::file_hash(resume_session.file_path.clone());

        if file_hash != resume_session.file_hash {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "File hash mismatch"));
        }

        let file_uploader = FileUploader {
            file_path: resume_session.file_path.clone(),
            file_size,
            container_size: resume_session.container_size,
            remaining_container_indexes: Arc::new(Mutex::new(VecDeque::from(resume_session.remaining_indexes.clone()))),
            current_downloading_indexes: Arc::new(Mutex::new(Vec::new())),
            containers: Arc::new(Mutex::new(resume_session.containers.clone())),
            pool: Arc::new(ThreadPool::new(resume_session.thread_count)),
            running: Default::default(),
        };

        Ok(file_uploader)
    }
}

struct FileThreadedUploader {
    current_container_index: Arc<Mutex<VecDeque<u32>>>,

    file_path: String,
    file_size: u64,
    container_size: u32,

    arguments: FileUploadArguments,

    client: Client,

    containers: Arc<Mutex<Vec<Container>>>,
    current_downloading_indexes: Arc<Mutex<Vec<u32>>>,

    running: Arc<RwLock<bool>>,
}

unsafe impl Send for FileThreadedUploader {}

impl FileThreadedUploader {
    fn new(current_container_index: Arc<Mutex<VecDeque<u32>>>,
           file_path: String,
           container_size: u32,
           arguments: FileUploadArguments,
           file_size: u64,
           containers: Arc<Mutex<Vec<Container>>>,
           current_downloading_indexes: Arc<Mutex<Vec<u32>>>,
           running: Arc<RwLock<bool>>,
    ) -> FileThreadedUploader {
        FileThreadedUploader {
            container_size,
            file_path,
            current_container_index,
            arguments,
            file_size,
            client: create_client(),
            containers,
            current_downloading_indexes,
            running,
        }
    }

    fn start_uploading(&mut self) {
        while let Some(container_index) = self.get_processing_container_index() {
            self.set_current_downloading_index(container_index);

            //println!("Uploading Container {:?}", container_index);

            if let Ok(container) = self.upload(container_index) {
                self.add_container(container);

                self.remove_current_downloading_index(container_index);
            }

            if !self.is_running() {
                break;
            }
        }

        return;
    }

    fn is_running(&self) -> bool {
        let lock = self.running.read().unwrap();

        *lock
    }

    fn upload(&mut self, container_index: u32) -> Result<Container, ()> {
        let filename = "data.enc".to_string();

        let mut salt = [0u8; 16];

        //println!("Doing upload of index {:?}", container_index);

        thread_rng().fill_bytes(&mut salt);

        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(self.arguments.encryption_password.as_bytes(), &salt, 10000, &mut key);


        //println!("Computing cursor chunks_per_container: {:?}", self.chunks_per_container());

        //let cursor = (((container_index - 1) * self.container_size) as i64) - ((METADATA_SIZE as i64) * (max(0, (container_index as i64) - 2)) * (self.chunks_per_container() as i64));
        let cursor = (container_index as i64 - 1) * self.chunks_per_container() as i64 * (CHUNK_SIZE as i64 - METADATA_SIZE as i64);

        //println!("cursor: {:?}", cursor);

        let remaining_real_size = self.file_size - cursor as u64;
        let remaining_extra_padding = ((remaining_real_size / (CHUNK_SIZE as u64 - METADATA_SIZE as u64)) + 1) * METADATA_SIZE as u64;

        //println!("Remaining real size: {:?} (extra padding {:?}", remaining_real_size, remaining_extra_padding);

        let mut remaining_size = min(self.container_size as u64, remaining_real_size + remaining_extra_padding);

        if remaining_size % (CHUNK_SIZE as u64) > 0 {
            remaining_size += (CHUNK_SIZE as u64) - remaining_size % (CHUNK_SIZE as u64);
        }

        //println!("Remaining size: {:?}", remaining_size);

        //println!("Requesting attachment");
        let (upload_url, upload_filename) = self.request_attachment(filename.clone(), remaining_size);

        //println!("Got upload url: {:?}", upload_url);

        let report_signal =
            if let Some(signal) = self.arguments.signal.clone() {
                let cursor_with_metadata = ((container_index as u64) - 1) * self.chunks_per_container() as u64 * (CHUNK_SIZE as u64);
                Some(Box::new(LinearPartSignal::new(signal.clone(), cursor_with_metadata)) as Box<dyn ReportSignal<u64>>)
            } else {
                None
            };

        let file_uploader = CustomBody::new(
            key,
            remaining_size as i64,
            self.file_path.clone(),
            cursor,
            report_signal,
            self.running.clone(),
        );

        let body = Body::sized(file_uploader, remaining_size);


        match self.client.put(upload_url)
            .header("accept-encoding", "gzip")
            .header("connection", "Keep-Alive")
            .header("content-length", remaining_size)
            .header("content-type", "application/x-x509-ca-cert")
            .header("host", "discord-attachments-uploads-prd.storage.googleapis.com")
            .header("user-agent", "Discord-Android/192013;RNA")
            .body(body).send() {
            Ok(_) => {
                let storage_url = self.post_message(filename.clone(), upload_filename);

                //println!("Computing byte range end (cursor: {:?}, remaining_size: {:?}, file_size {:?}, metadata size: {:?})", cursor, remaining_size, self.file_size, (remaining_size / CHUNK_SIZE as u64) * METADATA_SIZE as u64);
                let byte_range_end = min(self.file_size, cursor as u64 + remaining_size - ((remaining_size / CHUNK_SIZE as u64) * METADATA_SIZE as u64));

                Ok(Container {
                    storage_url,
                    chunk_count: remaining_size / CHUNK_SIZE as u64,
                    chunk_size: CHUNK_SIZE as u64,
                    salt,
                    bytes_range: [
                        cursor as u64,
                        byte_range_end
                    ],
                })
            }
            Err(_) => return Err(())
        }
    }

    fn get_processing_container_index(&mut self) -> Option<u32> {
        let mut deque = self.current_container_index.lock().unwrap();

        //println!("Trying to find work! (remaining indexes : {:?}", deque);

        deque.pop_front()
    }

    fn set_current_downloading_index(&mut self, index: u32) {
        let mut deque = self.current_downloading_indexes.lock().unwrap();

        deque.push(index);
    }

    fn remove_current_downloading_index(&mut self, index: u32) {
        let mut deque = self.current_downloading_indexes.lock().unwrap();

        deque.retain(|&x| x != index);
    }

    fn add_container(&mut self, container: Container) {
        let mut deque = self.containers.lock().unwrap();

        deque.push(container);
    }

    fn chunks_per_container(&self) -> u32 {
        self.container_size / CHUNK_SIZE
    }

    fn request_attachment(&self, filename: String, size: u64) -> (String, String) {
        //println!("Requesting attachment of size {:?}", size);

        let url = format!("https://discord.com/api/v9/channels/{}/attachments", self.arguments.channel_id);

        let payload = json!(
            {
                "files": [
                    {
                        "filename": filename,
                        "file_size": size,
                        "id": "8"
                    }
                ]
            }
        );


        let mut request = self.client.post(url);

        request = prepare_discord_request(request, self.arguments.token.clone());

        let resp = request.json(&payload).send().unwrap().json::<serde_json::Value>().unwrap();

        let upload_url = resp["attachments"][0]["upload_url"].as_str().unwrap();
        let upload_filename = resp["attachments"][0]["upload_filename"].as_str().unwrap();

        return (upload_url.to_string(), upload_filename.to_string());
    }

    fn post_message(&self, filename: String, upload_filename: String) -> String {
        // println!("Sending message with filename {:?} and upload_filename {:?}", filename, upload_filename);

        let url = format!("https://discord.com/api/v9/channels/{}/messages", self.arguments.channel_id);

        let payload = json!(
            {
                "content": "",
                "channel_id": self.arguments.channel_id,
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

        let req = self.client.post(url);

        let resp = prepare_discord_request(req, self.arguments.token.clone()).json(&payload)
            .send().unwrap().json::<serde_json::Value>().unwrap();

        let file_url = resp["attachments"][0]["url"].as_str().unwrap();

        //println!("Message has file url: {:?}", file_url);

        file_url.to_string()
    }
}


struct CustomBody {
    key: [u8; 32],

    remaining_size: i64,
    file: File,
    buffer_cursor: usize,
    buffer: Vec<u8>,

    signal: Option<Box<dyn ReportSignal<u64>>>,

    running: Arc<RwLock<bool>>,
}

unsafe impl Send for CustomBody {}

impl CustomBody {
    fn do_one_chunk(&mut self) {
        //  println!("Reading chunk (remaining to process: {:?})", self.remaining_size);

        let mut salt = [0u8; 16];
        thread_rng().fill_bytes(&mut salt);

        let content_size = min(self.remaining_size as usize, (CHUNK_SIZE as usize) - METADATA_SIZE);

        //  println!("Buffer size: {:?}, Content size {:?}", self.buffer.len(), content_size);

        self.file.read(&mut self.buffer[0..content_size]).unwrap();

        // println!("Read {:?} bytes from file", bytes_read);

        // compute hash
        let mut hasher = Sha256::new();
        hasher.update(&self.buffer[0..content_size]);
        let hash = hasher.finalize();

        //println!("Chunk hash: {:?}", hash);

        // encrypt data
        let cipher = Aes256Cbc::new_from_slices(
            &self.key.clone(),
            &salt.clone(),
        ).unwrap();

        // println!("Encryption key: {:?}", self.key.clone());
        // println!("Encryption salt: {:?}", salt.clone());

        // println!("Encrypting chunk from 0 to {:?}", content_size + 16);

        cipher.encrypt(&mut self.buffer[0..(content_size + 16)], content_size)
            .expect("encryption failure!");

        // println!("Setting salt at {:?} -> {:?}", (CHUNK_SIZE as usize) - 48, ((CHUNK_SIZE as usize) - 32));

        // add at end the iv
        self.buffer[(CHUNK_SIZE as usize) - 48..((CHUNK_SIZE as usize) - 32)].clone_from_slice(&salt.clone());

        self.buffer[(CHUNK_SIZE as usize) - 32..].clone_from_slice(&hash.clone());

        self.remaining_size -= CHUNK_SIZE as i64;
    }

    pub fn new(key: [u8; 32], remaining_size: i64, file_path: String, cursor: i64, signal: Option<Box<dyn ReportSignal<u64>>>, running: Arc<RwLock<bool>>) -> CustomBody {
        let mut file = File::open(file_path.clone()).unwrap();
        //println!("Seeking to {:?}", cursor);

        file.seek(SeekFrom::Current(cursor)).unwrap();

        CustomBody { key, remaining_size, file, buffer: vec![0; CHUNK_SIZE as usize], buffer_cursor: CHUNK_SIZE as usize, signal, running }
    }
}

impl Read for CustomBody {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read = 0;

        //println!("Doing read of {:?}", buf.len());

        while read < buf.len() {
            // println!("Read loop: buffer_cursor {:?} (read {:?})", self.buffer_cursor, read);

            if self.buffer_cursor < CHUNK_SIZE as usize {
                let remain = min(buf.len() - read, CHUNK_SIZE as usize - self.buffer_cursor);
                buf[read..(read + remain)].clone_from_slice(&self.buffer[self.buffer_cursor..(self.buffer_cursor + remain)]);
                // println!("Read loop: pushing {:?} buf", remain);
                read += remain;
                self.buffer_cursor += remain;
            }

            if self.buffer_cursor >= CHUNK_SIZE as usize {
                if self.remaining_size <= 0 {
                    //println!("End ! with read = {:?}", read);
                    break;
                } else {
                    //println!("Read loop: doing_one_chunk");

                    // before "processing next chunk", check if we are still allowed to run
                    {
                        let state = self.running.read().unwrap();
                        if !*state {
                            // println!("Detected stop state, aborting upload");
                            return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Upload aborted"));
                        }
                    }

                    self.do_one_chunk();
                    self.buffer_cursor = 0;
                }
            }
        }

        // report read;

        if let Some(signal) = self.signal.as_mut() {
            signal.report_data(read as u64);
        }

        Ok(read)
    }
}

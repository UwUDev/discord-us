use std::cmp::{min};
use std::fs::File;
use std::io::{Read, Write};
use aes::Aes256;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use reqwest::blocking::{Response};
use reqwest::{StatusCode};
use sha2::{Digest, Sha256};
use crate::common::{Container, Waterfall};
use crate::http_client::create_client;
use crate::signal::{ReportSignal, ProgressionRange, LinearPartSignal, PartProgression};

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

    //fn get_range(&self, start: u64, end: u64) -> ByteRangeStreamDownloader;
}


#[derive(Clone)]
pub struct DownloadProgressionSignal {
    signal: Option<Box<dyn ReportSignal<ProgressionRange<u64>>>>,
}

impl DownloadProgressionSignal {
    pub fn new() -> Self {
        Self {
            signal: None,
        }
    }

    fn get_report_signal(&self, cursor: u64) -> Option<Box<dyn ReportSignal<u64>>> {
        match &self.signal {
            Some(signal) => Some(Box::new(LinearPartSignal::new(
                signal.clone(),
                cursor,
            ))),
            None => None
        }
    }
}

#[derive(Clone)]
pub struct FileDownloader {
    waterfall: Waterfall,
    password: String,

    signal: DownloadProgressionSignal,
}

unsafe impl Send for FileDownloader {
}

impl WaterfallDownloader for FileDownloader {
    fn from_waterfall(waterfall: Waterfall) -> Self {
        let waterfall = waterfall.clone();
        let password = waterfall.password.clone();

        FileDownloader {
            waterfall,
            password,

            signal: DownloadProgressionSignal::new(),
        }
    }
}

impl FileDownloader {
    pub fn set_password(&mut self, password: String) -> &mut FileDownloader {
        self.password = password.clone();

        self
    }

    pub fn with_signal(&mut self, signal: &PartProgression<u64>) {
        self.signal.signal = Some(Box::new(signal.clone()));
    }

    pub fn get_container_downloader(&self, container: Container) -> ContainerDownloader {
        ContainerDownloader::new(container.clone(), self.waterfall.size, self.password.clone())
    }

    pub fn get_range(&self, start: u64, end: u64) -> ByteRangeStreamDownloader {
        let downloader = ByteRangeStreamDownloader::new([start, end], self.clone());

        downloader
    }
}

#[derive(Clone)]
pub struct ContainerDownloader {
    container: Container,
    key: [u8; 32],
    file_size: u64,
}

impl ContainerDownloader {
    fn hash_key(encryption_password: String, salt: [u8; 16]) -> [u8; 32] {
        let mut key = [0u8; 32];
        // unsafe ! pbkdf2::pbkdf2_hmac::<Sha256>(encryption_password.as_bytes(), &salt, 10000, &mut key);
        key
    }

    pub fn new(container: Container, file_size: u64, encryption_password: String) -> Self {
        let key = Self::hash_key(encryption_password, container.salt);

        ContainerDownloader {
            container,
            key,
            file_size,
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

        let response = create_client().get(container.storage_url.clone())
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

        let signal = &mut self.signal.get_report_signal(0);

        let mut containers = self.waterfall.clone().containers.clone();
        containers.sort_by(|a,b| a.bytes_range[0].cmp(&b.bytes_range[0]));

        for ctn in containers.iter() {
            let container = self.get_container_downloader(ctn.clone());
            let mut stream = container.get_byte_stream(0, ctn.chunk_count as usize).unwrap();

            let mut buf = [0u8; 65536 - 64];
            let mut to_write = (ctn.bytes_range[1] - ctn.bytes_range[0]) as usize;

            //println!("to_write: {}", to_write);

            while to_write > 0 {
                let read = stream.read(&mut buf).unwrap();

                let c = to_write.min(read);
                f.write_all(&mut buf[..c]).expect("TODO: panic message");
                // println!("to_write: {}", to_write);

                if let Some(s) = signal {
                    s.report_data(c as u64);
                }

                to_write -= c;
            }
        }
    }
}

impl ByteRangeDownloader for FileDownloader {
    fn get_size(&self) -> u64 {
        self.waterfall.size
    }
}

pub struct ByteRangeStreamDownloader {
    range: [u64; 2],
    file_downloader: FileDownloader,
    position: u64,
    current_container: Option<Container>,
    buffer: Vec<u8>,
    buffer_cursor: Option<usize>,

    current_bytestream: Option<ByteStream>,
    sorted_containers: Vec<Container>,
}

impl ByteRangeStreamDownloader {
    pub fn new(range: [u64; 2], file_downloader: FileDownloader) -> Self {
        let mut sorted_containers = file_downloader.waterfall.containers.clone();
        sorted_containers.sort_by(|a, b| a.bytes_range[0].cmp(&b.bytes_range[0]));

        ByteRangeStreamDownloader {
            range,
            file_downloader,

            position: range[0],
            current_container: None,
            buffer: Vec::new(),
            buffer_cursor: None,
            current_bytestream: None,

            sorted_containers,
        }
    }

    fn find_container(&self, start: u64) -> Option<Container> {
        for container in self.sorted_containers.clone() {
            if start >= container.bytes_range[0] && start < container.bytes_range[1] {
                return Some(container);
            }
        }
        None
    }

    fn get_remaining(&self) -> u64 {
        self.range[1] - self.position
    }

    fn read_all_into_buff(&mut self) -> usize {
        return match self.current_bytestream {
            Some(ref mut stream) => {
                let mut read = 0;

                while read < self.buffer.len() {
                    let r = stream.read(&mut self.buffer[read..]).expect("TODO: panic message");
                    read += r;
                }

                read
            }
            None => 0
        };
    }

    fn is_container_out_of_bound(&self) -> bool {
        return match self.current_container {
            Some(ref container) => {
                self.position >= container.bytes_range[1]
            }
            None => true
        };
    }

    fn is_position_out_of_bound(&self) -> bool {
        return self.position >= self.range[1];
    }

    fn update_container(&mut self) {
        self.current_container = self.find_container(self.position);
        //println!("Update container: {:?}", self.current_container);
    }

    fn start_container_download(&mut self) {
        if let Some(ref container) = self.current_container {
            let start = self.position - container.bytes_range[0];
            let chunk_size = self.get_chunk_real_size() as u64;
            // make start a multiple of chunk_size
            let chunk_start = start / chunk_size;
            let chunk_end = min(container.chunk_count, ((min(self.range[1], container.bytes_range[1]) - container.bytes_range[0]) / chunk_size) + 1);

            let container_downloader = self.file_downloader.get_container_downloader(container.clone());

            //println!("Starting container downloader start: {} || start : {} | end : {}", start, chunk_start, chunk_end);

            self.current_bytestream = container_downloader.get_byte_stream(chunk_start, (chunk_end - chunk_start) as usize).ok();
        }
    }

    fn get_buffer_cursor(&self) -> usize {
        return match self.buffer_cursor {
            Some(cursor) => cursor,
            None => 0
        };
    }

    fn set_buffer_cursor(&mut self, cursor: usize) {
        self.buffer_cursor = Some(cursor);
    }

    fn get_chunk_real_size(&self) -> usize {
        return match self.current_container {
            Some(ref container) => container.chunk_size as usize - METADATA_SIZE,
            None => 0
        };
    }

    fn get_current_chunk_size(&self, offset: usize) -> usize {
        let real_size = self.get_chunk_real_size();
        let remaining = self.get_remaining() as usize;

        return min(real_size, remaining + offset);
    }

    fn get_skip_offset(&self) -> usize {
        match self.current_container {
            Some(ref container) => {
                let chunk_size = self.get_chunk_real_size() as u64;

                // we are in the first chunk
                if self.position == self.range[0] {
                    let start = self.position - container.bytes_range[0];

                    let chunk_start = start / chunk_size;


                    return (self.position - (chunk_start * chunk_size)) as usize;
                }
            }
            None => {}
        }

        return 0;
    }
}

impl Read for ByteRangeStreamDownloader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read = 0;

        while read < buf.len() {
            let buffer_cursor = self.get_buffer_cursor();
            //println!("Read loop {} over {} (position {})", buffer_cursor, self.buffer.len(), self.position);

            if buffer_cursor < self.buffer.len() {
                let remain = min(buf.len() - read, self.buffer.len() - buffer_cursor);
                buf[read..(read + remain)].clone_from_slice(&self.buffer[buffer_cursor..(buffer_cursor + remain)]);
                read += remain;
                self.position += remain as u64;
                self.set_buffer_cursor(buffer_cursor + remain);
            }

            if self.is_position_out_of_bound() {
                break;
            }

            if self.is_container_out_of_bound() {
                //println!("Container out of bound");
                self.update_container();
                self.start_container_download();
            }

            if buffer_cursor >= self.buffer.len() {
                // load next chunk (shrink) into memory
                let offset = self.get_skip_offset();
                let size = self.get_current_chunk_size(offset);

                // if current buffer is not big enough, resize it
                if size != self.buffer.len() {
                    self.buffer = vec![0; size];
                }

                self.read_all_into_buff();


                //println!("Current size: {}, offset : {}", size, offset);

                self.buffer = self.buffer[offset..].to_vec();
                self.buffer_cursor = Some(0);
            }
        }

        Ok(read)
    }
}
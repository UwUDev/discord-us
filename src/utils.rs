extern crate aes;
extern crate block_modes;
extern crate pbkdf2;
extern crate hmac;
extern crate sha2;

use std::fs;
use aes::Aes256;
use block_modes::{BlockMode, Cbc};
use block_modes::block_padding::Pkcs7;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha2::Sha256;
use std::fs::File;
use std::io::{Read, Write};
use sha256::digest;
use uuid::Uuid;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

const SALT: &[u8] = b"miam le bon sel";
const TRASH_PATH: &str = "trash";

pub enum Subscription {
    Free,
    Basic,
    Classic,
    // yeah you can still buy it
    Boost,
}

#[derive(Debug)]
pub struct Block {
    pub num: u64,
    pub hash: String,
    pub size: u64,
    pub path: String,
    pub url: Option<String>,
}

pub fn to_blocks(input_file: &str, sub: Subscription) -> Vec<Block> {
    let max_chunk_size = get_max_chunk_upload_size(sub);
    let mut file = File::open(input_file).expect("Unable to open file");
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer).expect("Unable to read file");

    let mut blocks_paths = Vec::new();
    let mut block_num = 0;
    let mut block_start = 0;
    let mut block_end = max_chunk_size;

    let uuid = Uuid::new_v4();


    while block_start < buffer.len() {
        if block_end > buffer.len() {
            block_end = buffer.len();
        }
        let block = &buffer[block_start..block_end];
        let block_path = format!("{}/{}.{}.block", TRASH_PATH, uuid, block_num);
        let mut block_file = File::create(&block_path).expect("Unable to create block file");
        block_file.write_all(block).expect("Unable to write to block file");
        blocks_paths.push(block_path);
        block_num += 1;
        block_start = block_end;
        block_end += max_chunk_size;
    }

    block_num = 0;
    let mut blocks = Vec::new();
    for block_path in blocks_paths {
        let mut block_file = File::open(block_path.clone()).expect("Unable to open block file");
        let mut block_buffer = Vec::new();
        block_file.read_to_end(&mut block_buffer).expect("Unable to read block file");
        let block_hash = digest(&block_buffer);
        let block_size = block_buffer.len() as u64;
        let block = Block {
            num: block_num,
            hash: block_hash,
            size: block_size,
            path: block_path,
            url: None,
        };
        blocks.push(block);
        block_num += 1;
    }

    blocks
}

pub fn repack_blocks(blocks: Vec<Block>) -> String {
    let binding = blocks[0].path.clone();
    let output_file = binding.split(".").collect::<Vec<&str>>()[0];
    let mut output = File::create(output_file).expect("Unable to create output file");

    for block in blocks {
        let mut block_file = File::open(block.path.clone()).expect("Unable to open block file");
        let mut block_buffer = Vec::new();
        block_file.read_to_end(&mut block_buffer).expect("Unable to read block file");
        output.write_all(&block_buffer).expect("Unable to write to output file");
        fs::remove_file(block.path).expect("Unable to remove block file");
    }

    output_file.to_string()
}

pub fn encrypt_file(input_file: &str, output_file: &str, password: &str) {
    let mut file = File::open(input_file).expect("Unable to open file");
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer).expect("Unable to read file");

    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), SALT, 10000, &mut key);

    let cipher = Aes256Cbc::new_from_slices(&key, &key[..16]).unwrap();
    let encrypted_data = cipher.encrypt_vec(&buffer);

    let mut output = File::create(output_file).expect("Unable to create output file");
    output.write_all(&encrypted_data).expect("Unable to write to output file");
}

pub fn decrypt_file(input_file: &str, output_file: &str, password: &str) {
    let mut file = File::open(input_file).expect("Unable to open file");
    let mut encrypted_data = Vec::new();
    file.read_to_end(&mut encrypted_data).expect("Unable to read file");

    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), SALT, 10000, &mut key);

    let cipher = Aes256Cbc::new_from_slices(&key, &key[..16]).unwrap();
    let decrypted_data = cipher.decrypt_vec(&encrypted_data).unwrap();

    let mut output = File::create(output_file).expect("Unable to create output file");
    output.write_all(&decrypted_data).expect("Unable to write to output file");
}

pub fn calculate_file_md5(path: &str) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let digest = md5::compute(&buffer);
    let md5_string = format!("{:x}", digest);

    Ok(md5_string)
}

fn get_max_chunk_upload_size(subscription: Subscription) -> usize {
    match subscription {
        Subscription::Free => 24 * 1024 * 1024,
        Subscription::Basic => 50 * 1024 * 1024,
        Subscription::Classic => 100 * 1024 * 1024,
        Subscription::Boost => 500 * 1024 * 1024,
    }
}

pub fn empty_trash() {
    let paths = fs::read_dir(TRASH_PATH).unwrap();

    for path in paths {
        let path = path.unwrap().path();
        fs::remove_file(path).expect("Unable to remove file");
    }
}

pub fn create_trash_dir() {
    fs::create_dir_all(TRASH_PATH).expect("Unable to create trash dir");
}
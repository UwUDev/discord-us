use std::fs::File;
use std::io::Write;
use std::time::Duration;
use reqwest::blocking::Client;
use serde_json::json;
use sha256::digest;
use crate::database::save_upload;
use crate::utils::{Block, calculate_file_md5, empty_trash, encrypt_file, Subscription, to_blocks};

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
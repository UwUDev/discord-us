use sha256::digest;
use crate::database::{get_blocks, get_file_md5, get_file_name, get_hashed_pass};
use crate::utils::{Block, calculate_file_md5, decrypt_file, repack_blocks};

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
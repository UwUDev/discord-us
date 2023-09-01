use std::fs::File;
use std::io::{BufReader, Write};
use rusqlite::{Connection, params};
use serde_json::{Map, Number, Value};
use sha256::digest;
use crate::utils::Block;

pub fn save_upload(file_name: &str, size: u64, md5: &str, hashed_pass: &str, block_count: usize, blocks: &Vec<Block>) -> usize {
    let conn = Connection::open("files.db").unwrap();
    let sql = format!(
        "INSERT INTO files (name, md5, size, blocks, hashed_pass) VALUES ('{}', '{}', {}, {}, '{}')",
        file_name, md5, size, block_count, hashed_pass
    );
    conn.execute(
        sql.as_str(),
        [],
    ).unwrap();

    let saved_id = conn.last_insert_rowid() as usize;

    for block in blocks {
        let sql = format!(
            "INSERT INTO blocks (file_id, block_num, block_hash, block_size, url) VALUES ((SELECT id FROM files WHERE md5 = '{}'), {}, '{}', {}, '{}')",
            md5, block.num, block.hash, block.size, block.url.clone().unwrap()
        );
        conn.execute(
            sql.as_str(),
            [],
        ).unwrap();
    }

    saved_id
}

pub fn create_db(default_pass: &str) {
    let conn = Connection::open("files.db").unwrap();

    let default_hashed_pass = digest(default_pass);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            md5 VARCHAR(32) NOT NULL,
            size INTEGER NOT NULL,
            blocks INTEGER NOT NULL,
            hashed_pass VARCHAR(64) NOT NULL
        )",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS blocks (
            file_id INTEGER NOT NULL,
            block_num INTEGER NOT NULL,
            block_hash TEXT NOT NULL,
            block_size INTEGER NOT NULL,
            url TEXT NOT NULL,
            FOREIGN KEY (file_id) REFERENCES files(id)
        )",
        [],
    ).unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS infos (
            default_hashed_pass VARCHAR(64) NOT NULL
        )",
        [],
    ).unwrap();

    conn.execute(
        "DELETE FROM infos",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO infos (default_hashed_pass) VALUES (?1)",
        [default_hashed_pass],
    ).unwrap();
}

pub fn get_file_name(id: usize) -> String {
    let conn = Connection::open("files.db").unwrap();
    let mut stmt = conn.prepare(
        "SELECT name FROM files WHERE id = ?1"
    ).unwrap();

    let mut name = String::new();
    let name_iter = stmt.query_map(
        [id],
        |row| {
            Ok(row.get(0).unwrap())
        },
    ).unwrap();

    for n in name_iter {
        name = n.unwrap();
    }

    name
}

pub fn get_blocks(id: usize) -> Vec<Block> {
    let conn = Connection::open("files.db").unwrap();
    let mut stmt = conn.prepare(
        "SELECT block_num, block_hash, block_size, url FROM blocks WHERE file_id = ?1"
    ).unwrap();

    let mut blocks = Vec::new();
    let block_iter = stmt.query_map(
        [id],
        |row| {
            Ok(Block {
                num: row.get(0).unwrap(),
                hash: row.get(1).unwrap(),
                size: row.get(2).unwrap(),
                url: Some(row.get(3).unwrap()),
                path: String::new(),
            })
        },
    ).unwrap();

    for block in block_iter {
        blocks.push(block.unwrap());
    }

    blocks
}

pub fn get_file_md5(id: usize) -> String {
    let conn = Connection::open("files.db").unwrap();
    let mut stmt = conn.prepare(
        "SELECT md5 FROM files WHERE id = ?1"
    ).unwrap();

    let mut md5 = String::new();
    let md5_iter = stmt.query_map(
        [id],
        |row| {
            Ok(row.get(0).unwrap())
        },
    ).unwrap();

    for md5_ in md5_iter {
        md5 = md5_.unwrap();
    }

    md5
}

pub fn get_hashed_pass(id: usize) -> String {
    let conn = Connection::open("files.db").unwrap();
    let mut stmt = conn.prepare(
        "SELECT hashed_pass FROM files WHERE id = ?1"
    ).unwrap();

    let mut hashed_pass = String::new();
    let hashed_pass_iter = stmt.query_map(
        [id],
        |row| {
            Ok(row.get(0).unwrap())
        },
    ).unwrap();

    for hashed_pass_ in hashed_pass_iter {
        hashed_pass = hashed_pass_.unwrap();
    }

    hashed_pass
}

fn get_full_file(id: usize) -> (String, String, usize, usize, String) {
    let conn = Connection::open("files.db").unwrap();
    let mut stmt = conn.prepare(
        "SELECT name, md5, size, blocks, hashed_pass FROM files WHERE id = ?1"
    ).unwrap();

    let mut name = String::new();
    let mut md5 = String::new();
    let mut size = 0;
    let mut block_count = 0;
    let mut hashed_pass = String::new();
    let file_iter = stmt.query_map(
        [id],
        |row| {
            Ok((
                row.get(0).unwrap(),
                row.get(1).unwrap(),
                row.get(2).unwrap(),
                row.get(3).unwrap(),
                row.get(4).unwrap(),
            ))
        },
    ).unwrap();

    for file in file_iter {
        let file_ = file.unwrap();
        name = file_.0;
        md5 = file_.1;
        size = file_.2;
        block_count = file_.3;
        hashed_pass = file_.4;
    }

    (name, md5, size, block_count, hashed_pass)
}


pub fn export_waterfall(id: usize, path: &str) {
    let mut waterfall = Map::new();
    let file = get_full_file(id);
    let blocks = get_blocks(id);

    waterfall.insert("name".to_string(), Value::String(file.0));
    waterfall.insert("md5".to_string(), Value::String(file.1));
    waterfall.insert("size".to_string(), Value::Number(Number::from(file.2)));
    waterfall.insert("blocks".to_string(), Value::Number(Number::from(file.3)));
    waterfall.insert("hashed_pass".to_string(), Value::String(file.4));

    let mut block_array = Vec::new();
    for block in blocks {
        let mut block_map = Map::new();
        block_map.insert("num".to_string(), Value::Number(Number::from(block.num)));
        block_map.insert("hash".to_string(), Value::String(block.hash));
        block_map.insert("size".to_string(), Value::Number(Number::from(block.size)));
        block_map.insert("url".to_string(), Value::String(block.url.unwrap()));
        block_array.push(Value::Object(block_map));
    }

    waterfall.insert("block_array".to_string(), Value::Array(block_array));

    let mut file = File::create(path).unwrap();
    file.write_all(serde_json::to_string_pretty(&waterfall).unwrap().as_bytes()).unwrap();

    println!("Exported waterfall to {}", path);
}

pub fn import_waterfall(path: &str) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let waterfall: Map<String, Value> = serde_json::from_reader(reader).unwrap();

    let name = waterfall.get("name").unwrap().as_str().unwrap();
    let md5 = waterfall.get("md5").unwrap().as_str().unwrap();
    let size = waterfall.get("size").unwrap().as_u64().unwrap() as usize;
    let blocks = waterfall.get("blocks").unwrap().as_u64().unwrap() as usize;
    let hashed_pass = waterfall.get("hashed_pass").unwrap().as_str().unwrap();

    let conn = Connection::open("files.db").unwrap();
    conn.execute(
        "INSERT INTO files (name, md5, size, blocks, hashed_pass) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, md5, size, blocks, hashed_pass],
    ).unwrap();

    let id = conn.last_insert_rowid() as usize;

    let block_array = waterfall.get("block_array").unwrap().as_array().unwrap();
    for block in block_array {
        let num = block.get("num").unwrap().as_u64().unwrap() as usize;
        let hash = block.get("hash").unwrap().as_str().unwrap();
        let size = block.get("size").unwrap().as_u64().unwrap() as usize;
        let url = block.get("url").unwrap().as_str().unwrap();

        conn.execute(
            "INSERT INTO blocks (file_id, block_num, block_hash, block_size, url) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, num, hash, size, url],
        ).unwrap();
    }

    println!("Imported waterfall from {}", path);
}
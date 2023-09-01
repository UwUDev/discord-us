use rusqlite::Connection;
use sha256::digest;
use crate::utils::Block;

pub fn save_upload(file_name: &str, size: u64, md5: &str, hashed_pass: &str, block_count: usize, blocks: &Vec<Block>) {
    let conn = Connection::open("files.db").unwrap();
    let sql = format!(
        "INSERT INTO files (name, md5, size, blocks, hashed_pass) VALUES ('{}', '{}', {}, {}, '{}')",
        file_name, md5, size, block_count, hashed_pass
    );
    conn.execute(
        sql.as_str(),
        [],
    ).unwrap();

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
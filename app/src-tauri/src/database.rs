use std::sync::{Mutex, MutexGuard};
use rusqlite::{Connection, Transaction, vtab::array};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State, command};
use crate::state::{AppInitializer, AppDirectory, AppState};

const DB_VERSION: u32 = 1;

pub struct Database {
    pub connection: Connection,
}

pub trait Versionned {
    fn get_current_version(&self) -> u32;

    fn set_current_version(&mut self, version: u32);

    fn get_last_version(&self) -> u32;

    fn upgrade(&mut self, version: u32);

    fn upgrade_if_needed(&mut self) {
        let last_version = self.get_last_version();
        let mut current_version = self.get_current_version();

        while current_version < last_version {
            current_version += 1;

            self.upgrade(current_version);

            self.set_current_version(current_version);
        }
    }
}

impl AppInitializer for Database {
    fn init(app_handle: &AppHandle) -> Self {
        let sqlite_path = app_handle.get_app_data_dir().join("db.sqlite");

        let connection = Connection::open(sqlite_path).unwrap();

        array::load_module(&connection).unwrap();

        let mut db = Database {
            connection,
        };

        db.upgrade_if_needed();

        db
    }
}

impl Versionned for Database {
    fn get_current_version(&self) -> u32 {
        let mut query = self.connection.prepare("PRAGMA user_version").unwrap();

        query.query_row([], |row| row.get(0)).unwrap_or(0)
    }

    fn set_current_version(&mut self, version: u32) {
        self.connection.pragma_update(None, "user_version", version).unwrap();
    }

    fn get_last_version(&self) -> u32 {
        DB_VERSION
    }

    fn upgrade(&mut self, version: u32) {
        let tx = self.connection.transaction().unwrap();

        match version {
            1 => {
                tx.execute_batch(
                    "
                    CREATE TABLE options (
                        name VARCHAR(255) PRIMARY KEY,
                        value TEXT NOT NULL
                    );

                    CREATE TABLE items (
                        id INT PRIMARY KEY,
                        name TEXT NOT NULL,
                        status INT NOT NULL,

                        added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    );
                    "
                ).unwrap();
            },
            _ => {}
        }

        tx.commit().unwrap();
    }
}

#[derive(Serialize, Debug)]
pub enum ItemStatus {
    DOWNLOADING,
    UPLOADING,
    DONE,
}

impl ItemStatus {
    fn to_code(&self) -> i32 {
        match self {
            ItemStatus::DOWNLOADING => 0,
            ItemStatus::UPLOADING => 1,
            ItemStatus::DONE => 2,
        }
    }

    fn from_code(value: i32) -> ItemStatus {
        match value {
            0 => ItemStatus::DOWNLOADING,
            1 => ItemStatus::UPLOADING,
            2 => ItemStatus::DONE,
            _ => ItemStatus::DOWNLOADING,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Item {
    id: i32,
    name: String,
    status: ItemStatus,
}

#[command]
pub fn get_items (state: State<'_, AppState>, filter: Option<&str>) -> Vec<Item> {
    let mut database = state.database.lock().unwrap();

    let mut database = database.as_mut().unwrap();

    let mut stmt = database.connection.prepare("SELECT * FROM items WHERE name = coalesce(?, name)").unwrap();

    let items = stmt.query_map([filter], |row| {
        Ok(Item {
            id: row.get(0)?,
            name: row.get(1)?,
            status: ItemStatus::from_code(row.get(2)?),
        })
    }).unwrap().map(|item| item.unwrap()).collect();

    println!("Items (filter={:?}): {:?}", filter, items);

    items
}

#[command]
pub fn get_options(state: State<'_, AppState>, options: Vec<String>) -> Vec<Option<String>> {
    let mut database = state.database.lock().unwrap();

    let mut database = database.as_mut().unwrap();

    let mut stmt = database.connection.prepare("SELECT name, value FROM options WHERE name IN rarray(?1)").unwrap();

    let values:Vec<rusqlite::types::Value> = options.clone().into_iter().map(rusqlite::types::Value::from).collect();
    let ptr = std::rc::Rc::new(values);

    let opts : Vec<(String, String)> = stmt.query_map(&[&ptr], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).unwrap().map(|item| item.unwrap()).collect();

    let mut res: Vec<Option<String>> = Vec::with_capacity(options.len());

    for i in 0..options.len() {
        let o = &options[i];
        if let Some(v) = opts.iter().find(|(name, _)| name == o).map(|(_, value)| value) {
            res.push(Some(v.to_string()));
        } else {
            res.push(None);
        }
    }

   // println!("Loaded {:?} => {:?}", options, res);

    res
}

#[command]
pub fn set_options(state: State<'_, AppState>, options: Vec<String>) {
    let mut database = state.database.lock().unwrap();

    let mut database = database.as_mut().unwrap();

    let mut tx = database.connection.transaction().unwrap();

    let mut stmt = tx.prepare("INSERT OR REPLACE INTO options (name, value) VALUES (?, ?)").unwrap();

    for i in (0..options.len()).step_by(2) {
        let o = &options[i];
        stmt.execute([o, &options[i + 1]]).unwrap();
    }

    drop(stmt);

    tx.commit().unwrap();

   // println!("Saved {:?}", options);
}

#[command]
pub fn get_option (state: State<'_, AppState>, option: String) -> Option<String> {
    _get_option(&state.database.lock().unwrap().as_ref().unwrap(), &option)
}

pub fn _get_option (db: &Database, name: &String) -> Option<String> {
    let mut stmt = db.connection.prepare("SELECT value FROM options WHERE name = ?").unwrap();

    stmt.query_row([name], |row| row.get(0)).ok()
}
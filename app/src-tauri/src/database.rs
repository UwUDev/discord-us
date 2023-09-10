use std::sync::{Mutex, MutexGuard};
use rusqlite::{Connection, Error, params, Transaction, vtab::array};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State, command};
use crate::state::{AppInitializer, AppDirectory, AppState};

const DB_VERSION: u32 = 3;

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
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        name TEXT NOT NULL,
                        status INT NOT NULL,

                        added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    );
                    "
                ).unwrap();
            }
            2 => {
                tx.execute_batch(
                    "
                    ALTER TABLE items ADD COLUMN resume_data TEXT DEFAULT NULL;
                    ALTER TABLE items ADD COLUMN progression_data TEXT DEFAULT NULL;
                    ALTER TABLE items ADD COLUMN user_password BOOLEAN DEFAULT FALSE;
                    ALTER TABLE items ADD COLUMN password TEXT DEFAULT NULL;
                    ALTER TABLE items ADD COLUMN thread_count INT DEFAULT 1;
                    ALTER TABLE items ADD COLUMN file_path TEXT DEFAULT NULL;
                    "
                ).unwrap()
            },
            3 => {
                tx.execute_batch(
                    "
                    ALTER TABLE items ADD COLUMN deleted_at TIMESTAMP DEFAULT NULL;
                    "
                ).unwrap()
            }
            _ => {}
        }

        tx.commit().unwrap();
    }
}

#[derive(Serialize, Debug, Clone)]
pub enum ItemStatus {
    DOWNLOADING,
    UPLOADING,
    DONE,
}

impl ItemStatus {
    pub fn to_code(&self) -> i32 {
        match self {
            ItemStatus::DOWNLOADING => 0,
            ItemStatus::UPLOADING => 1,
            ItemStatus::DONE => 2,
        }
    }

    pub fn from_code(value: i32) -> ItemStatus {
        match value {
            0 => ItemStatus::DOWNLOADING,
            1 => ItemStatus::UPLOADING,
            2 => ItemStatus::DONE,
            _ => ItemStatus::DOWNLOADING,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub status: ItemStatus,

    pub file_path: String,
    pub user_password: bool,
    pub password: String,
    pub progression_data: Option<String>,
    pub resume_data: Option<String>,
    pub thread_count: u32,
}

#[command]
pub fn get_items(state: State<'_, AppState>, filter: Option<&str>) -> Vec<Item> {
    let mut database = state.database.lock().unwrap();

    let mut database = database.as_mut().unwrap();

    let (query, params) = match filter {
        Some(filter) => {
           ( "SELECT * FROM items WHERE name LIKE '%' || ?1 || '%' AND deleted_at IS NULL", [filter].to_vec())
        }
        None => {
            ("SELECT * FROM items WHERE deleted_at IS NULL", Vec::new())
        }
    };

    let mut stmt = database.connection.prepare(query).unwrap();
    let items = stmt.query_map(rusqlite::params_from_iter(params), |row| import_row_as_item(row)).unwrap().map(|item| item.unwrap()).collect();


    items
}

fn import_row_as_item(row: &rusqlite::Row) -> Result<Item, Error> {
    Ok(Item {
        id: row.get(0)?,
        name: row.get(1)?,
        status: ItemStatus::from_code(row.get(2)?),
        resume_data: row.get(4)?,
        progression_data: row.get(5)?,
        password: row.get(7)?,
        user_password: row.get(6)?,
        thread_count: row.get(8)?,
        file_path: row.get(9)?,
    })
}

#[command]
pub fn get_item(state: State<'_, AppState>, id: i32) -> Result<Item, String> {
    let database = state.database.lock().unwrap();

    let database = database.as_ref().unwrap();

    println!("Get item {}", id);

    _get_item(&database, id).map_err(|e| e.to_string())
}

pub fn _get_item(database: &Database, id: i32) -> Result<Item, Error> {
    let mut stmt = database.connection.prepare("SELECT * FROM items WHERE id = ?").unwrap();

    stmt.query_row([id], |row| import_row_as_item(row))
}

pub fn _get_items_with_status(database: &Database, status: ItemStatus) -> Result<Vec<Item>, Error> {
    let mut stmt = database.connection.prepare("SELECT * FROM items WHERE status = ? AND deleted_at IS NULL").unwrap();

    stmt.query_map([status.to_code()], |row| import_row_as_item(row))
        .map(|rows| rows.map(|i| i.unwrap()))
        .map(|rows| rows.collect())
}

pub fn _update_item(database: &Database, id: i32, progression_data: Option<String>, resume_data: Option<String>, status: ItemStatus) {
    let mut stmt = database.connection.prepare("UPDATE items SET progression_data = ?, resume_data = ?, status = ? WHERE id = ?").unwrap();

    stmt.execute(rusqlite::params![progression_data, resume_data, status.to_code(), id]).unwrap();
}

pub fn notify_item_updated(database: &Database, id: i32, app_handle: &AppHandle) {
    use tauri::Manager;

    if let Ok(item) = _get_item(database, id) {
        app_handle.emit_all("push_item", item).unwrap();
    }
}

pub fn _delete_item(database: &Database, id: i32) {
    let mut stmt = database.connection.prepare("UPDATE items SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?").unwrap();

    stmt.execute(rusqlite::params![id]).unwrap();
}

#[command]
pub fn get_options(state: State<'_, AppState>, options: Vec<String>) -> Vec<Option<String>> {
    let mut database = state.database.lock().unwrap();

    let mut database = database.as_mut().unwrap();

    let mut stmt = database.connection.prepare("SELECT name, value FROM options WHERE name IN rarray(?1)").unwrap();

    let values: Vec<rusqlite::types::Value> = options.clone().into_iter().map(rusqlite::types::Value::from).collect();
    let ptr = std::rc::Rc::new(values);

    let opts: Vec<(String, String)> = stmt.query_map(&[&ptr], |row| {
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
pub fn get_option(state: State<'_, AppState>, option: String) -> Option<String> {
    _get_option(&state.database.lock().unwrap().as_ref().unwrap(), &option)
}

pub fn _get_option(db: &Database, name: &String) -> Option<String> {
    let mut stmt = db.connection.prepare("SELECT value FROM options WHERE name = ?").unwrap();

    stmt.query_row([name], |row| row.get(0)).ok()
}
use rusqlite::{Connection};
use tauri::{AppHandle, State};
use crate::state::{AppInitializer, AppDirectory};

const CURRENT_DB_VERSION: u32 = 1;

struct Database {}

impl AppInitializer for Database {
    fn init(app_handle: &AppHandle) -> Self {
        let sqlite_path = app_handle.get_app_data_dir().join("db.sqlite");

        let mut db = Connection::open(sqlite_path);

        Database {}
    }
}
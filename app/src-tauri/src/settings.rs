use std::fs;
use std::fs::{read_to_string, write};
use tauri::command;

const SETTINGS : Settings<'static> = Settings {
    file_path: &"settings.json",
};

struct Settings<'a> {
    file_path: &'a str
}

impl<'a> Settings<'a> {
    fn read_settings(&self) -> Option<String> {
        let contents = read_to_string(self.file_path);

        contents.ok()
    }

    fn write_settings(&self, contents: String) {
        fs::metadata(self.file_path).ok().map(|d| {
            println!("File exists: {}", d.is_file());
        });

        write(self.file_path, contents).unwrap();
    }
}

#[command]
pub fn get_settings() -> Option<String> {
    SETTINGS.read_settings()
}

#[command]
pub fn save_settings(settings: String) {
    SETTINGS.write_settings(settings);
}
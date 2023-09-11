use std::{
    fs::File,
    ops::Range,
    path::PathBuf,
};
use walkdir::WalkDir;
use crate::{
    fs::{
        FsReadable,
        file::{
            FileReadable
        },
    },
    utils::range::Ranged,
};

struct DirEntry {
    file: FileReadable,
    relative_path: String,

    range: Range<u64>,
}

impl Ranged for DirEntry {
    fn get_range(&self) -> &Range<u64> {
        &self.range
    }
}

fn scan_files(files: Vec<PathBuf>) -> Result<Vec<DirEntry>, walkdir::Error>{
    let mut cursor = 0;

    let mut dir_entries: Vec<DirEntry> = Vec::new();

    for file in files.iter() {
        for fw in WalkDir::new(file) {
            let entry = fw.unwrap();

            let relative_path = entry.path().strip_prefix(file);

            let size = entry.metadata().unwrap().len();

            // dir_entries.push(DirEntry {
            //     file: FileReadable::new(entry.path()),
            //     relative_path: relative_path.to_str().unwrap().to_string(),
            //     range: cursor..cursor + size,
            // });

            cursor += size;
        }
    }

    Ok(dir_entries)
}
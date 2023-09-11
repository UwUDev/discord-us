use crate::{
    Size,
    fs::{FsReadable},
};
use std::{
    fs::File,
    io::{Read, Seek},
    path::PathBuf,
};

pub struct FileReadable {
    path: PathBuf,
    inner: File,
}

impl Size for File {
    fn get_size(&self) -> u64 {
        self.metadata().unwrap().len()
    }
}

impl Size for FileReadable {
    fn get_size(&self) -> u64 {
        self.inner.get_size()
    }
}

impl Clone for FileReadable {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            inner: File::open(self.path.clone()).unwrap(),
        }
    }
}

impl Read for FileReadable {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for FileReadable {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

unsafe impl Send for FileReadable {}

impl FsReadable for FileReadable {}
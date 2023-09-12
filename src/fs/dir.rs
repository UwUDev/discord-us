use std::{
    fs::{File, canonicalize},
    ops::Range,
    path::PathBuf,
    io::{Read, SeekFrom},
};
use walkdir::WalkDir;
use crate::{
    utils::{
        range::Ranged,
        read::{
            Chunked,
            LazyOpen,
            RangeLazyOpen,
            ChunkSize,
        },
    },
};

#[derive(Clone, Debug)]
struct DirEntry {
    path: PathBuf,

    range: Range<u64>,
}

impl Ranged for DirEntry {
    fn get_range(&self) -> &Range<u64> {
        &self.range
    }
}

fn scan_files(files: Vec<PathBuf>) -> Result<Vec<DirEntry>, std::io::Error> {
    let mut cursor = 0;

    let mut dir_entries: Vec<DirEntry> = Vec::new();

    for file in files.iter() {
        let file = canonicalize(file)?;
        for fw in WalkDir::new(&file) {
            let entry = fw?;

            let metadata = entry.metadata().unwrap();

            let size = if metadata.is_dir() {
                0
            } else {
                metadata.len()
            };

            let path = match entry.path() {
                path if path.is_absolute() => path.to_path_buf(),
                path => file.join(path),
            };

            dir_entries.push(DirEntry {
                path,
                range: cursor..cursor + size,
            });

            cursor += size;
        }
    }

    Ok(dir_entries)
}

const FILE_READER_CHUNK_SIZE: usize = 2048;

struct ChunkedFileReader {
    file: File,
}

impl ChunkedFileReader {
    fn open(path: &PathBuf, pos: SeekFrom) -> Self {
        let file = File::open(path).unwrap();

        Self {
            file,
        }
    }
}

impl Chunked for ChunkedFileReader {
    fn process_next_chunk(&mut self) -> Option<Vec<u8>> {
        let mut buf = [0u8; FILE_READER_CHUNK_SIZE];

        let read = self.file.read(&mut buf).unwrap();

        #[cfg(test)]
        println!("Processing next chunk for file (read={}) {:?}", read, self.file);

        match read {
            0 => None,
            read => Some(buf[..read].to_vec())
        }
    }
}

impl ChunkSize for DirEntry {
    fn get_chunk_size(&self) -> u64 {
        FILE_READER_CHUNK_SIZE as u64
    }
}

impl LazyOpen<ChunkedFileReader> for DirEntry {
    fn open(&self) -> ChunkedFileReader {
        ChunkedFileReader::open(&self.path, SeekFrom::Start(0))
    }
}

impl RangeLazyOpen<ChunkedFileReader> for DirEntry {
    fn open_with_range(&self, range: Range<u64>) -> ChunkedFileReader {
        ChunkedFileReader::open(&self.path, SeekFrom::Start(range.start))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        Size,
        fs::dir::{
            scan_files,
            ChunkedFileReader, DirEntry,
        },
        utils::{
            read::{
                MultiChunkedStream,
                MultiChunkedReader,
                LazyOpen,
            }
        },
    };
    use std::{
        io::{
            Read
        }
    };

    #[test]
    pub fn test_scan() {
        let files = scan_files(vec!["./src".into()]).unwrap();

        println!("Scanned {} files", files.len());

        println!("{:?}", files);
    }

    #[test]
    pub fn test_read() {
        let files = scan_files(vec!["./src".into()]).unwrap();

        let mut size = files.get_size();

        println!("Size: {}", size);

        let stream: MultiChunkedStream<DirEntry, ChunkedFileReader> = files.into();

        let mut r = stream.open();

        let mut buf = [0u8; 2048];

        while size > 0 {
            let read = r.read(&mut buf).unwrap();
            println!("Read {} bytes|r {}", read, size);
            size -= read as u64;
        }
    }
}
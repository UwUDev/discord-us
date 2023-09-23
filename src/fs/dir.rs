use std::{
    fs::{File, canonicalize},
    ops::Range,
    path::PathBuf,
    io::{Read, SeekFrom},
};
use std::io::Seek;
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
    fs::{
        FsNode,
        IntoTree,
        Ref,
        AsPathRelative,
    },
};
use serde::{
    Serialize, Deserialize,
};

#[derive(Clone, Debug)]
pub struct DirEntry {
    path: PathBuf,

    range: Range<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirEntryNode {
    range: Range<u64>,
}

impl Ranged for DirEntry {
    fn get_range(&self) -> &Range<u64> {
        &self.range
    }
}

pub fn scan_files(files: Vec<PathBuf>) -> Result<Vec<DirEntry>, std::io::Error> {
    let mut cursor = 0;

    let mut dir_entries: Vec<DirEntry> = Vec::new();

    for file in files.iter() {
        let file = canonicalize(file)?;
        for fw in WalkDir::new(&file) {
            let entry = fw?;

            let metadata = entry.metadata()?;

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

pub struct ChunkedFileReader {
    file: File,
}

impl ChunkedFileReader {
    fn open(path: &PathBuf, pos: SeekFrom) -> Self {
        let mut file = File::open(path).unwrap();

        file.seek(pos).unwrap();

        Self {
            file,
        }
    }
}

impl Chunked for ChunkedFileReader {
    fn process_next_chunk(&mut self) -> Option<Vec<u8>> {
        let mut buf = [0u8; FILE_READER_CHUNK_SIZE];

        let read = self.file.read(&mut buf).unwrap();

        //#[cfg(test)]
        //println!("Processing next chunk for file (read={}) {:?}", read, self.file);

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

impl IntoTree<DirEntryNode, &Vec<String>> for &Vec<DirEntry> {
    fn into_tree(&self, prefix: &Vec<String>) -> Ref<FsNode<DirEntryNode>> {
        let root = FsNode::root();

        for entry in self.iter() {
            let path = entry.path.as_path_relative(prefix);

            let node = root.borrow_mut().find_recursive_create(&path);

            node.borrow_mut().set_data(DirEntryNode {
                range: entry.range.clone(),
            });
        }

        root
    }
}

#[cfg(test)]
mod test {
    use crate::{
        Size,
        fs::{AsPathVec, SerializedFsNode, IntoTree, dir::{
            scan_files,
            ChunkedFileReader, DirEntry,
            DirEntryNode,
        }},
        utils::{
            read::{
                MultiChunkedStream,
                RangeLazyOpen,
            }
        },
    };
    use std::{
        io::{
            Read
        },
        ops::{Range},
    };
    use std::io::Write;
    use std::path::{PathBuf};

    #[test]
    pub fn test_scan() {
        let scan: PathBuf = "./src".into();

        let files = scan_files(vec![scan.clone()]).unwrap();

        println!("Scanned {} files", files.len());

        let relative = scan.canonicalize().unwrap().as_path_vec();
        println!("Relative: {:?}", relative);

        // for file in files.iter() {
        //   println!("Path {:?}", file.path.as_path_relative(&relative).as_path_string());
        //}

        let node = (&files).into_tree(&relative);

        let node = (*node.borrow()).clone();

        let str = serde_json::to_string(&node).unwrap();
        println!("{}", str);

        let node: SerializedFsNode<DirEntryNode> = serde_json::from_str(&str).unwrap();

        println!("{:?}", node.into_node(None));
    }

    #[test]
    pub fn test_read() {
        let files = scan_files(vec!["./Cargo.toml".into(), "./README.md".into()]).unwrap();

        let rg = Range { start: 0, end: files.get_size() };
        let mut size = rg.end - rg.start;

        let stream: MultiChunkedStream<DirEntry, ChunkedFileReader> = files.into();

        let mut r = stream.open_with_range(rg);

        let mut buf = [0u8; 2048];
        let mut f = std::fs::File::create("test.txt").unwrap();

        while size > 0 {
            let read = r.read(&mut buf).unwrap();
            println!("Read {} bytes|r {}", read, size);
            f.write(&buf[..read]).unwrap();
            size -= read as u64;
        }
    }
}
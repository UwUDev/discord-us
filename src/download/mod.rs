pub mod http;

use std::{
    io::{Read, Result},
    ops::{Range},
};

use crate::{
    pack::{
        Size,
        container::{
            Container,
        },
        crypt,
        crypt::{
            ChunkCipher,
        },
        key::{
            KeyDerivator,
        },
    },
    signal::{
        AddSignaler,
        DerivedSignal,
        StoredSignal,
        progress::{
            ProgressSignal,
            ProgressSignalTrait,
        },
    },
    utils::{
        read::{
            RangeLazyOpen,
            LazyOpen,
            ReadProxy,
            Chunked,
            ChunkSize,
            MultiChunkedStream,
        },
        safe::{Safe},
        range::{Ranged},
    },
    download::{
        http::{
            HttpDownloader,
        }
    },
};

pub trait Download<R: Read> {
    fn download(&self) -> Result<R>;
}

#[derive(Clone)]
pub struct ContainerOpener {
    container: Container,
    signal: ProgressSignal<StoredSignal<Vec<Range<u64>>>>,
    key_derivator: KeyDerivator,
}

pub struct OpenedContainer<T: Read, S: AddSignaler<Range<u64>>> {
    container: Container,
    read_total: u64,
    reader: T,
    signal: ProgressSignal<S>,
    cipher: ChunkCipher,
}

type SignalRange = DerivedSignal<Safe<StoredSignal<Vec<Range<u64>>>>, u64>;
type OpenedC = OpenedContainer<ReadProxy, SignalRange>;

impl ContainerOpener {
    pub fn new(container: Container, signal: ProgressSignal<StoredSignal<Vec<Range<u64>>>>, password: String) -> Self {
        let key_derivator = KeyDerivator::from_password(password);
        Self {
            container,
            signal,
            key_derivator,
        }
    }
}

impl ChunkSize for ContainerOpener {
    fn get_chunk_size(&self) -> u64 {
        self.container.meta.chunk_size - crypt::METADATA_SIZE
    }
}

impl Ranged for ContainerOpener {
    fn get_range(&self) -> &Range<u64> {
        &self.container.meta.bytes_range
    }
}

impl LazyOpen<OpenedC> for ContainerOpener {
    fn open(&self) -> OpenedC {
        self.open_with_range(0..self.container.meta.get_size())
    }
}

impl RangeLazyOpen<OpenedC> for ContainerOpener {
    fn open_with_range(&self, range: Range<u64>) -> OpenedC {
        //#[cfg(test)]
        //println!("Opening container ({}-{}) (#size: {}) with range {:?}", self.container.meta.bytes_range.start, self.container.meta.bytes_range.end, self.container.meta.get_size(), range);

        let chunk_count_end = range.end / (self.get_chunk_size());
        let chunk_count_start = range.start / (self.get_chunk_size());
        let mut range = (range.start + chunk_count_start * crypt::METADATA_SIZE)..(range.end + chunk_count_end * crypt::METADATA_SIZE);

        let key = self.key_derivator.derive_password(&self.container.meta.salt);

        let cipher = ChunkCipher::new(&key);

        // check protocol ? for the moment, only http
        let downloader = HttpDownloader::new(self.container.public_url.clone(), range.clone());

        let stream = downloader.download().unwrap();

        OpenedContainer {
            container: self.container.clone(),
            read_total: 0,
            reader: stream,
            signal: self.signal.clone_with_offset(self.container.meta.bytes_range.start),
            cipher,
        }
    }
}

impl<T: Read, S: AddSignaler<Range<u64>>> Chunked for OpenedContainer<T, S> {
    fn process_next_chunk(&mut self) -> Option<Vec<u8>> {
        let chunk_size = self.container.meta.chunk_size as usize;

        // let read 1 chunk
        let mut buf = vec![0u8; chunk_size];

        let mut read = 0;

        //#[cfg(test)]
        //println!("Process_next_Chunk ({}-{}) {} {}",self.container.meta.bytes_range.start, self.container.meta.bytes_range.end, self.read_total, self.read_total/self.container.meta.chunk_size);

        while read < chunk_size {
            if !self.signal.is_running() {
                return None;
            }

            let r = self.reader.read(&mut buf[read..]).unwrap();

            if r == 0 {
                return None;
            }

            self.signal.add_signal(self.read_total..(r as u64 + self.read_total));

            read += r;
            self.read_total += r as u64;
        }

        // once the chunk is read, decrypt it
        self.cipher.decrypt(&mut buf).ok()?;

        Some(buf[12..chunk_size - 16].to_vec())
    }
}

pub struct ContainerDownloader {
    containers: Vec<Container>,
    password: String,
}

impl ContainerDownloader {
    pub fn new(containers: Vec<Container>, password: String) -> Self {
        Self { containers, password }
    }

    pub fn to_stream(&self, signal: ProgressSignal<StoredSignal<Vec<Range<u64>>>>) -> MultiChunkedStream<ContainerOpener, OpenedC> {
        let containers: Vec<_> = self.containers.iter().map(|x|
            ContainerOpener::new(x.clone(), signal.clone(), self.password.clone())
        ).collect();

        MultiChunkedStream::from(containers)
    }
}

#[cfg(test)]
mod test {
    use crate::{pack::{
        Waterfall,
        SerializableWaterfall,
    }, download::{
        ContainerDownloader,
    }, utils::{
        read::{RangeLazyOpen}
    }, Size, ZeroSubstract};
    use std::io::{Read, SeekFrom, Write, Seek};
    use std::thread::JoinHandle;

    use serde_json;
    use crate::utils::read::LazyOpen;

    use std::os::windows::fs::FileExt;

    #[test]
    pub fn test() {
        let serializable_waterfall: SerializableWaterfall = serde_json::from_reader(std::fs::File::open("test.json").unwrap()).unwrap();

        let waterfall = Waterfall::from_serializable(serializable_waterfall);

        let mut signal = crate::signal::progress::ProgressSignal::<crate::signal::StoredSignal<Vec<std::ops::Range<u64>>>>::new();

        let mut downloader = ContainerDownloader::new(waterfall.containers, "password".to_string());

        let stream = downloader.to_stream(signal.clone());

        let mut handles: Vec<JoinHandle<()>> = Vec::new();

        let t_count = 3;

        let l = stream.get_size() / t_count;

        for i in 0..t_count {
            let range = i * l..stream.get_size().min((i + 1) * l);
            let s = stream.clone();

            handles.push(std::thread::spawn(move || {
                println!("Downloading range {:?}", range);

                let mut reader = s.open_with_range(range.clone());

                let mut buf = vec![0u8; 1 << 16];

                let mut read = 0;

                let mut f = std::fs::File::create(format!("testC.{}.txt", i)).unwrap();
                //f.seek(SeekFrom::Start(range.start)).unwrap();
                println!("Seeked to {:?}", f.stream_position());

                while read < range.get_size() {
                    let c = reader.read(&mut buf).unwrap();
                    read += c as u64;
                    // println!("position to {} (+= {})", f.stream_position().unwrap(), c);
                    f.write(&buf[0..c]).unwrap();
                }

                println!("Wrote from {} to {}", range.start, range.start + read);
            }));

           // handles.pop().unwrap().join().unwrap();
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}

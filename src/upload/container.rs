use std::{io::{Error, Read}, ops::{Range}, thread::{
    ScopedJoinHandle
}, collections::VecDeque, thread};
use std::ops::Deref;
use std::sync::{Arc, Barrier, MutexGuard};
use std::thread::sleep;
use crate::{
    utils::{
        safe::{Safe, SafeAccessor},
        read::{
            RangeLazyOpen,
            ChunkedRead,
        },
    },
    pack::{
        container::{
            Container, PartialContainer,
            ChunkSplitter,
        },
        crypt,
    },
    signal::{
        AddSignaler,
        progress::{
            ProgressSignal,
            ProgressSignalTrait,
        },
    },
    upload::{
        Uploader,
        UploaderMaxSize,
    },
    Size,
};
use crate::pack::crypt::METADATA_SIZE;
use crate::signal::bool::SafeBoolSignal;
use crate::signal::{Signaler, SignalValue};
use crate::utils::read::LazyOpen;

pub struct ContainerUploader<U: Uploader<String, ChunkedRead<crypt::StreamCipher<R>>, S> + Clone, R: Read, S: AddSignaler<Range<u64>>> {
    container_size: u64,
    chunk_size: u64,

    password: String,

    remaining_containers: Safe<VecDeque<Range<u64>>>,
    containers: Safe<Vec<Container>>,

    uploader: U,

    thread_count: usize,

    _phantom: std::marker::PhantomData<(R, S)>,
}

impl<U: Uploader<String, ChunkedRead<crypt::StreamCipher<R>>, S> + Clone, R: Read, S: AddSignaler<Range<u64>>> ContainerUploader<U, R, S> {
    pub fn new(
        container_size: u64,
        chunk_size: u64,
        password: String,
        uploader: U,
        thread_count: usize,
    ) -> Self {
        Self {
            container_size,
            chunk_size,
            password,
            remaining_containers: Safe::wrap(VecDeque::new()),
            containers: Safe::wrap(Vec::new()),
            uploader,
            thread_count,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn upload<X: RangeLazyOpen<R> + Clone>(&mut self, reader: X, size: u64, signal: &mut ProgressSignal<S>) -> Result<Vec<Container>, Error> {
        {
            let mut remaining_containers = self.remaining_containers.access();

            if remaining_containers.is_empty() {
                let splitter = ChunkSplitter::new(self.chunk_size, crypt::METADATA_SIZE, self.container_size);
                *remaining_containers = splitter.split_into_ranges(size).into();
            }
        }

        thread::scope(|s| {
            let mut join_handles: Vec<ScopedJoinHandle<'_, ()>> = Vec::new();
            let mut running = SafeBoolSignal::new(false);

            for _ in 0..self.thread_count {
                let mut worker_thread = WorkerThread::new(
                    self.uploader.clone(),
                    reader.clone(),
                    self.remaining_containers.clone(),
                    signal.clone(),
                    self.containers.clone(),
                    ChunkSplitter::new(self.chunk_size, crypt::METADATA_SIZE, self.container_size),
                    self.password.clone(),
                    running.clone(),
                );


                join_handles.push(s.spawn(move || {
                    worker_thread.run();
                }));
            }

            for join_handle in join_handles {
                let _ = join_handle.join();
            }
        });


        Ok(self.containers.access().clone())
    }
}

impl<U: Uploader<String, ChunkedRead<crypt::StreamCipher<SeqReader<R>>>, S> + Clone, R: Read, S: AddSignaler<Range<u64>>> ContainerUploader<U, SeqReader<R>, S> {
    /// Upload sequential is a less optimized version of upload
    /// taking only a sequential stream as input and needs buffering to improve operation speeds
    pub fn upload_seq(&mut self, reader: R, signal: &mut ProgressSignal<S>) -> Result<Vec<Container>, Error> {
        thread::scope(|s| {
            let mut join_handles: Vec<ScopedJoinHandle<'_, ()>> = Vec::new();
            let mut running = SafeBoolSignal::new(true);

            let read_opener = SeqReaderOpener {
                pos: Safe::wrap(0),
                stream: Safe::wrap(reader),
                lock: SafeBoolSignal::new(false),
                end: SafeBoolSignal::new(false),
                payload_size: (self.chunk_size as usize - METADATA_SIZE as usize),
                end_at: Safe::wrap(0),
            };

            for _ in 0..self.thread_count {
                let mut worker_thread = WorkerThread::new(
                    self.uploader.clone(),
                    read_opener.clone(),
                    self.remaining_containers.clone(),
                    signal.clone(),
                    self.containers.clone(),
                    ChunkSplitter::new(self.chunk_size, crypt::METADATA_SIZE, self.container_size),
                    self.password.clone(),
                    running.clone(),
                );


                join_handles.push(s.spawn(move || {
                    worker_thread.run();
                }));
            }

            let mut pos = 0;
            let chunk_splitter = ChunkSplitter::new(self.chunk_size, crypt::METADATA_SIZE, self.container_size);
            loop {
                if !signal.is_running() || read_opener.end.get_value() {
                    #[cfg(test)]
                    println!("Stopping");

                    break;
                }
                let locked = read_opener.lock.get_value();
                if !locked {
                    let mut remaining_containers = self.remaining_containers.access();
                    #[cfg(test)]
                    println!("Checking if empty | end {} | containersize {} | pos {}", read_opener.end.get_value(), remaining_containers.len(), pos);
                    if *read_opener.pos.access() >= pos as u64 {
                        if remaining_containers.is_empty() {
                            #[cfg(test)]
                            println!("Adding new range {:?}", pos..(pos + chunk_splitter.max_payload_size()));
                            remaining_containers.push_back(
                                pos..(pos + chunk_splitter.max_payload_size())
                            );
                            pos += chunk_splitter.max_payload_size();
                        }
                    }
                }
                sleep(std::time::Duration::from_millis(50));
            }
            running.signal(false);

            for join_handle in join_handles {
                let _ = join_handle.join();
            }

            let end_at = read_opener.end_at.access();

            let mut containers = self.containers.access();
            let l = containers.len();
            containers.sort_by(|a, b| a.meta.bytes_range.start.cmp(&b.meta.bytes_range.start));
            if l > 0 {
                let last = containers.get_mut(l - 1).unwrap();
                last.meta.bytes_range.end = *end_at;
                last.meta.chunk_count = ((*end_at - last.meta.bytes_range.start) / (self.chunk_size))+1;
                //last.meta.bytes_range.end = *end_at;

            }
        });

        Ok(self.containers.access().clone())
    }
}

pub struct SeqReader<R: Read> {
    stream: Safe<R>,
    remaining: u64,
    lock: SafeBoolSignal,
    end: SafeBoolSignal,
    end_at: Safe<u64>,
    pos: Safe<u64>,
    payload_size: usize,
}

impl<X: Read> Read for SeqReader<X> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            #[cfg(test)]
            println!("End of stream");
            self.lock.signal(false);

            return Ok(0);
        }

        let read = buf.len().min(self.remaining as usize);

        let mut read = self.stream.access().read(&mut buf[..read])?;
        if read == 0 {

            // filling with 0 untils remaining==0;
            // TODO: smarter filling, until remaining is a multiple of payload size
            // read = self.payload_size;
            read = buf.len().min((self.remaining as i64 - self.payload_size as i64).abs() as usize % self.payload_size);

            let mut end_at = self.end_at.access();
            if *end_at == 0 {
                *end_at = *self.pos.access();
            }

            #[cfg(test)]
            println!("Premature end of stream >& {} | read={} | pos {}", self.remaining, read, self.pos.access());
            self.end.signal(true);
        }
        *self.pos.access() += read as u64;
        self.remaining -= (read as u64).min(self.remaining);

        if self.remaining == 0 {
            #[cfg(test)]
            println!("End of stream");
            self.lock.signal(false);
        }

        Ok(read)
    }
}


struct SeqReaderOpener<R: Read> {
    pos: Safe<u64>,
    stream: Safe<R>,
    lock: SafeBoolSignal,
    end: SafeBoolSignal,
    end_at: Safe<u64>,
    payload_size: usize,
}

impl<R: Read> Clone for SeqReaderOpener<R> {
    fn clone(&self) -> Self {
        Self {
            pos: self.pos.clone(),
            stream: self.stream.clone(),
            lock: self.lock.clone(),
            end: self.end.clone(),
            payload_size: self.payload_size,
            end_at: self.end_at.clone(),
        }
    }
}

impl<R: Read> LazyOpen<SeqReader<R>> for SeqReaderOpener<R> {
    fn open(&self) -> SeqReader<R> {
        todo!()
    }
}

impl<R: Read> RangeLazyOpen<SeqReader<R>> for SeqReaderOpener<R> {
    fn open_with_range(&self, range: Range<u64>) -> SeqReader<R> {
        #[cfg(test)]
        println!("Opening range {:?}", range);
        loop {
            let locked = self.lock.get_value();

            if locked {
                continue;
            }

            let mut pos = self.pos.access();
            if *pos == range.start {
                #[cfg(test)]
                println!("Getting exclusive access to {:?}", range);
                return SeqReader {
                    stream: self.stream.clone(),
                    remaining: range.get_size(),
                    lock: self.lock.clone(),
                    end: self.end.clone(),
                    pos: self.pos.clone(),
                    payload_size: self.payload_size,
                    end_at: self.end_at.clone(),
                };
            }
            thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

impl<U: Uploader<String, ChunkedRead<crypt::StreamCipher<R>>, S> + Clone, R: Read, S: AddSignaler<Range<u64>>> UploaderMaxSize for ContainerUploader<U, R, S> {
    fn get_max_size(&self) -> u64 {
        self.container_size
    }
}

struct WorkerThread<U: Uploader<String, ChunkedRead<crypt::StreamCipher<R>>, S> + Clone, R: Read, X: RangeLazyOpen<R>, S: AddSignaler<Range<u64>>> {
    uploader: U,
    reader: X,

    progress_signal: ProgressSignal<S>,

    remaining_containers: Safe<VecDeque<Range<u64>>>,
    containers: Safe<Vec<Container>>,
    splitter: ChunkSplitter,
    password: String,

    _phantom: std::marker::PhantomData<(R, S)>,
    running: SafeBoolSignal,
}

unsafe impl<U: Uploader<String, ChunkedRead<crypt::StreamCipher<R>>, S> + Clone, R: Read, X: RangeLazyOpen<R>, S: AddSignaler<Range<u64>>> Send for WorkerThread<U, R, X, S> {}

impl<U: Uploader<String, ChunkedRead<crypt::StreamCipher<R>>, S> + Clone, R: Read, X: RangeLazyOpen<R>, S: AddSignaler<Range<u64>>> WorkerThread<U, R, X, S> {
    pub fn new(
        uploader: U,
        reader: X,
        remaining_containers: Safe<VecDeque<Range<u64>>>,
        progress_signal: ProgressSignal<S>,
        containers: Safe<Vec<Container>>,
        splitter: ChunkSplitter,
        password: String,
        running: SafeBoolSignal,
    ) -> Self {
        Self {
            uploader,
            reader,
            progress_signal,
            remaining_containers,
            containers,
            splitter,
            password,
            _phantom: std::marker::PhantomData,
            running,
        }
    }

    pub fn add_uploaded_containers(&mut self, c: Vec<Container>) {
        let mut containers = self.containers.access();
        let mut remaining_containers = self.remaining_containers.access();

        containers.extend(c);
        remaining_containers.retain(|r| !containers.iter().any(|c| c.meta.bytes_range == *r));
    }

    fn run(&mut self) {
        #[cfg(test)]
        println!("Worker thread started");
        while self.progress_signal.is_running() {
            let mut remaining_containers = self.remaining_containers.access();

            // #[cfg(test)]
            // println!("Remaining containers: {:?}", remaining_containers);

            if let Some(range) = remaining_containers.pop_front().clone() {
                drop(remaining_containers);
                #[cfg(test)]
                println!("Doing range upload {:?}", range);
                self.upload_range(range);
            } else {
                // println!("No more ranges to upload, sleeping >> {}", self.running.get_value());
                if self.running.get_value() {
                    drop(remaining_containers);
                    sleep(std::time::Duration::from_millis(50));
                } else {
                    break;
                }
            }
        }
    }

    fn upload_range(&mut self, range: Range<u64>) {
        let read = self.reader.open_with_range(range.clone());

        let padded_range = self.splitter.add_padding(&range);

        println!("Upload range {:?} (padded: {:?})", range, padded_range);

        let container = PartialContainer::new_container(
            self.splitter.chunk_size,
            self.splitter.max_size,
            range.clone(),
            self.password.clone(),
        ).unwrap();

        let stream: ChunkedRead<crypt::StreamCipher<R>> = container.encrypt_stream(read).into();

        match self.uploader.do_upload(
            stream,
            padded_range.get_size(),
            &mut self.progress_signal.clone().into(),
        ) {
            Ok(url) => {
                self.containers.access().push(container.into_container(url.unwrap()));
            }
            Err(e) => {
                let mut remaining_containers = self.remaining_containers.access();
                eprintln!("{:?}", e);
                eprintln!("Error uploading range {:?}, retrying", range);
                remaining_containers.push_back(range); // if an error occured: retry
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;
    use std::ops::{Range};
    use std::path::PathBuf;
    use crate::{
        upload::{
            pool::{UploadPool},
            bot::{BotUploader},
            account::{AccountCredentials, AccountSubscription},
            container::{ContainerUploader},
        },
        signal::{
            progress::{
                ProgressSignal,
                ProgressSignalTrait,
            },
            StoredSignal,
        },
        fs::{
            dir::{scan_files, DirEntry, ChunkedFileReader},
            IntoTree,
            AsPathVec,
        },
        utils::{
            read::{MultiChunkedStream},
            safe::{SafeAccessor},
        },
        Size,
        pack::{
            Waterfall,
            SerializableWaterfall,
        },
    };
    use crate::fs::FsNode;
    use crate::signal::progress::ProgressSignalAccessor;
    use crate::signal::StaticSignal;
    use crate::upload::webhook::WebhookUploader;
    use crate::utils::read::StaticStream;

    #[test]
    pub fn test_webhook() {
        let tokens = std::env::var("TOKENS").map(|t| t.split(",").map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();
        let mut pool = UploadPool::new();
        for token in tokens.clone() {
            let (id, token) = token.split_once(":").unwrap();
            let id = id.parse::<u64>().unwrap();
            println!("ID: {}, Token: {}", id, token);
            let mut up = WebhookUploader::new(AccountCredentials {
                channel_id: id,
                access_token: token.into(),
                subscription: AccountSubscription::Free,
            });
            up.include_token(true);
            pool.add_uploader(up);
        }

        let mut uploader = ContainerUploader::new(2 * 1024 * 1024,
                                                  1 << 16, // 65536 bytes
                                                  "password".into(),
                                                  pool.clone(),
                                                  5);

        let signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();

        let f = scan_files(vec!["./testc.mp4".into()]).unwrap();

        let path: PathBuf = "./upl".into();

        let tree = (&f).into_tree(&path.canonicalize().unwrap().as_path_vec());

        let r: MultiChunkedStream<DirEntry, ChunkedFileReader> = f.into();

        println!("Size: {}", r.get_size());
        let start = std::time::Instant::now();

        let ss = signal.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let mut p = ss.get_progression().access();
                p.retrim_ranges();
                println!("Prog {:?}", p.get_signal_data());
            }
        });

        let r = uploader.upload(r.clone(), r.get_size(), &mut signal.clone().into()).unwrap();
        println!("Elapsed: {:?}", start.elapsed());
        println!("Containers: {:?}", r);

        let file = std::fs::File::create("test.json").unwrap();

        let w = Waterfall::new(tree, r);

        serde_json::to_writer_pretty(file, &w.as_serializable()).unwrap();
    }

    struct TestRead2 {
        remaining: u64,
    }
    impl Read for TestRead2 {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let read = buf.len().min(self.remaining as usize);
            buf.fill(0x1);
            self.remaining -= read as u64;
            Ok(read)
        }
    }

    #[test]
    pub fn test_webhook2() {
        let tokens = std::env::var("TOKENS").map(|t| t.split(",").map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();
        let mut pool = UploadPool::new();
        for token in tokens {
            let (id, token) = token.split_once(":").unwrap();
            let id = id.parse::<u64>().unwrap();
            println!("ID: {}, Token: {}", id, token);
            let mut up = WebhookUploader::new(AccountCredentials {
                channel_id: id,
                access_token: token.into(),
                subscription: AccountSubscription::Free,
            });
            up.include_token(true);
            pool.add_uploader(up);
        }
        // Test2
        let mut uploader = ContainerUploader::new(2 * 1024 * 1024,
                                                  1 << 16, // 65536 bytes
                                                  "password".into(),
                                                  pool.clone(),
                                                  5);

        let signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();
        let r = uploader.upload_seq(
            std::fs::File::open("./testc.mp4").unwrap(), &mut signal.clone()).unwrap();

        let file = std::fs::File::create("test.json").unwrap();

        let wf = Waterfall::new(FsNode::root(), r);
        serde_json::to_writer_pretty(file, &wf.as_serializable()).unwrap();
    }


    #[test]
    pub fn test() {
        let tokens = std::env::var("TOKENS").map(|t| t.split(",").map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();

        let mut pool = UploadPool::new();

        for token in tokens {
            pool.add_uploader(BotUploader::new(AccountCredentials {
                channel_id: 1200557435572715610,
                access_token: token.into(),
                subscription: AccountSubscription::Free,
            }));
        }

        let signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();

        let f = scan_files(vec!["./upl".into()]).unwrap();

        let path: PathBuf = "./src".into();

        let tree = (&f).into_tree(&path.canonicalize().unwrap().as_path_vec());

        let r: MultiChunkedStream<DirEntry, ChunkedFileReader> = f.into();

        println!("Size: {}", r.get_size());

        let mut u = ContainerUploader::new(
            AccountSubscription::Free.get_max_upload_size() as u64,
            1 << 16, // 65536 bytes
            "password".into(),
            pool,
            5,
        );

        let mut s = signal.clone();

        // std::thread::spawn(move || {
        //     std::thread::sleep(std::time::Duration::from_secs(5));
        //     println!("Stopping");
        //     s.stop();
        // });

        let r = u.upload(r.clone(), r.get_size(), &mut signal.clone().into()).unwrap();

        let v = u.remaining_containers.access().clone();

        println!("Containers: {:?}", r);
        println!("Remaining: {:?}", v);
        let mut p = signal.get_progression().access();
        p.retrim_ranges();
        println!("Signal data {:?}", p.get_signal_data());

        for a in v.iter() {
            println!("Size: {}", a.get_size());
        }

        let file = std::fs::File::create("test.json").unwrap();

        let w = Waterfall::new(tree, r);

        serde_json::to_writer_pretty(file, &w.as_serializable()).unwrap();
    }
}
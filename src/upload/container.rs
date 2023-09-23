use std::{io::{Error, Read}, ops::{Range}, thread::{
    ScopedJoinHandle
}, collections::VecDeque, thread};
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

    fn upload<X: RangeLazyOpen<R> + Clone>(&mut self, reader: X, size: u64, signal: &mut ProgressSignal<S>) -> Result<Vec<Container>, Error> {
        {
            let mut remaining_containers = self.remaining_containers.access();

            if remaining_containers.is_empty() {
                let splitter = ChunkSplitter::new(self.chunk_size, crypt::METADATA_SIZE, self.container_size);
                *remaining_containers = splitter.split_into_ranges(size).into();
            }
        }

        thread::scope(|s| {
            let mut join_handles: Vec<ScopedJoinHandle<'_, ()>> = Vec::new();

            for _ in 0..self.thread_count {
                let mut worker_thread = WorkerThread::new(
                    self.uploader.clone(),
                    reader.clone(),
                    self.remaining_containers.clone(),
                    signal.clone(),
                    self.containers.clone(),
                    ChunkSplitter::new(self.chunk_size, crypt::METADATA_SIZE, self.container_size),
                    self.password.clone(),
                );


                join_handles.push(s.spawn(move || {
                    worker_thread.run();
                }));
            }

            for join_handle in join_handles {
                join_handle.join().unwrap();
            }
        });


        Ok(self.containers.access().clone())
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
        }
    }

    fn run(&mut self) {
        #[cfg(test)]
        println!("Worker thread started");
        while self.progress_signal.is_running() {
            let mut remaining_containers = self.remaining_containers.access();

            #[cfg(test)]
            println!("Remaining containers: {:?}", remaining_containers);

            if let Some(range) = remaining_containers.pop_front().clone() {
                drop(remaining_containers);
                self.upload_range(range);
            } else {
                break;
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
            range,
            self.password.clone(),
        ).unwrap();

        let stream: ChunkedRead<crypt::StreamCipher<R>> = container.encrypt_stream(read).into();

        if let Ok(url) = self.uploader.do_upload(
            stream,
            padded_range.get_size(),
            &mut self.progress_signal.clone().into(),
        ) {
            self.containers.access().push(container.into_container(url.unwrap()));
        } else {
            /*let mut remaining_containers = self.remaining_containers.access();
            remaining_containers.push_back(range); // if an error occured: retry

             */
        }

    }
}

#[cfg(test)]
mod test {
    use std::ops::{Range};
    use crate::{
        upload::{
            pool::{UploadPool},
            bot::{BotUploader},
            account::{AccountCredentials, AccountSubscription},
            container::{ContainerUploader},
        },
        signal::{
            progress::{
                ProgressSignal
            },
            StoredSignal,
        },
        fs::{
            dir::{scan_files, DirEntry, ChunkedFileReader},
        },
        utils::{
            read::{MultiChunkedStream},
        },
        Size,
    };

    #[test]
    pub fn test() {
        let tokens = std::env::var("TOKENS").map(|t| t.split(",").map(|s| s.to_string()).collect::<Vec<_>>()).unwrap();

        let mut pool = UploadPool::new();

        for token in tokens {
            pool.add_uploader(BotUploader::new(AccountCredentials {
                channel_id: 1147616702780149781,
                access_token: token.into(),
                subscription: AccountSubscription::Free,
            }));
        }

        let signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();

        let f = scan_files(vec!["./cli/JellyfinMediaPlayer-1.9.1-windows-x64.exe".into()]).unwrap();

        let r: MultiChunkedStream<DirEntry, ChunkedFileReader> = f.into();

        println!("Size: {}", r.get_size());

        let mut u = ContainerUploader::new(
            AccountSubscription::Free.get_max_upload_size() as u64,
            1 << 16, // 65536 bytes
            "password".into(),
            pool,
            5,
        );

        let r = u.upload(r.clone(), r.get_size(), &mut signal.clone().into()).unwrap();

        println!("Containers: {:?}", r);
    }
}
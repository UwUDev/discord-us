use std::{
    io::{Error, Read},
    ops::Range,
    thread::sleep,
    time::{Duration, Instant},
    cell::RefCell,
};
use dyn_clone::{clone_trait_object, DynClone};

use crate::{
    upload::{
        Uploader,
        UploaderMaxSize,
        UploaderCoolDownResponse,
    },
    signal::{
        AddSignaler,
        progress::{ProgressSignal},
    },
    utils::{
        limit::{
            CoolDown,
            CoolDownMs,
        },
        safe::{
            Safe,
            SafeAccessor,
        },
    },
};

pub trait ClonableUploader<V, R: Read, S: AddSignaler<Range<u64>>> : DynClone + Uploader<V, R, S> {}

clone_trait_object!(<V, R, S> ClonableUploader<V, R, S> where R: Read, S: AddSignaler<Range<u64>>);

impl<T: DynClone + Uploader<V, R, S>, V, R: Read, S: AddSignaler<Range<u64>>> ClonableUploader<V, R, S> for T {}

pub struct PooledUploader<S: AddSignaler<Range<u64>>, R: Read> {
    uploader: Box<dyn ClonableUploader<String, R, S>>,
    cooldown: CoolDown,
    max_size: u64,

    _phantom: std::marker::PhantomData<(R, S)>,
}

impl<S: AddSignaler<Range<u64>>, R: Read> PooledUploader<S, R> {
    pub fn new<U: Uploader<String, R, S> + CoolDownMs + Clone + 'static>(uploader: U) -> Self {
        Self {
            cooldown: uploader.create_cooldown_wait(),
            max_size: uploader.get_max_size(),
            uploader: Box::new(uploader),

            _phantom: std::marker::PhantomData,
        }
    }
}

pub struct UploadPool<S: AddSignaler<Range<u64>>, R: Read> {
    uploaders: Safe<Vec<RefCell<PooledUploader<S, R>>>>,
}

impl<S: AddSignaler<Range<u64>>, R: Read> Clone for UploadPool<S, R> {
    fn clone(&self) -> Self {
        Self {
            uploaders: self.uploaders.clone(),
        }
    }
}

unsafe impl<S: AddSignaler<Range<u64>>, R: Read> Send for UploadPool<S, R> {}

impl<S: AddSignaler<Range<u64>>, R: Read> UploadPool<S, R> {
    pub fn new() -> Self {
        Self {
            uploaders: Safe::wrap(Vec::new()),
        }
    }

    pub fn add_uploader<U: Uploader<String, R, S> + CoolDownMs + Clone + 'static>(&mut self, uploader: U) {
        self.uploaders.access().push(RefCell::new(PooledUploader::new(uploader)));
    }
}

impl<S: AddSignaler<Range<u64>>, R: Read> UploadPool<S, R> {
    fn next_uploader(&self, uploaders: &Vec<RefCell<PooledUploader<S, R>>>) -> Option<usize> {
        // TO THINK: take other data for the computation of the best uploader
        // like X-RateLimit-Remaining for bot requests
        uploaders.iter()
            .enumerate()
            .filter(|(_,x)| !x.borrow().cooldown.is_working())
            .min_by_key(|(_, x)| {
                let cell = x.borrow();
                if cell.cooldown.is_working() {
                    u64::MAX
                } else {
                    cell.cooldown.remaining_wait()
                }
            }).map(|(i, _)| i)
    }

    fn _do_upload(
        &self,
        reader: R,
        size: u64,
        signal: ProgressSignal<S>,
    ) -> Result<UploaderCoolDownResponse<String>, Error> {
        // acquire lock on uploaders
        loop {
            let uploaders = self.uploaders.access();
            println!("Acquired lock on uploaders");

            // find next best uploader
            let uploader_index = self.next_uploader(&uploaders);

            // if no index is found => wait 50ms and retry
            if uploader_index.is_none() {
                drop(uploaders);
                sleep(Duration::from_millis(50));
                continue;
            }

            let uploader_index = uploader_index.unwrap();

            let mut uploader = uploaders.get(uploader_index)
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "No uploader available"))?
                .borrow_mut();

            println!("StartWork {} | uploader.is_working: {}", uploader_index, uploader.cooldown.is_working());
            uploader.cooldown.start_work(); // << mark this uploader as start working even if we aren't working rn

            println!("Waiting for cooldown {}ms ({})", uploader.cooldown.remaining_wait(), uploader_index);

            // clone uploader so we can drop the lock on uploaders
            let mut cooldown_wait_clone = uploader.cooldown.clone();
            let mut upl = uploader.uploader.clone();

            drop(uploader);
            drop(uploaders); // << release the lock on uploaders


            cooldown_wait_clone.wait(); // wait for cooldown

            let result = upl.do_upload(reader, size, signal)?;

            let ended_at = Instant::now();

            println!("Finished uploading, acquiring lock on uploaders ({})", uploader_index);

            // acquire lock on uploaders
            let uploaders = self.uploaders.access();
            let mut uploader = uploaders.get(uploader_index)
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "An error occured: cannot unlock uploader"))?
                .borrow_mut();

            if let Some(cooldown_adapt) = result.get_cooldown() {
                println!("Cooldown adapt {}ms ({})", cooldown_adapt, uploader_index);
                uploader.cooldown.set_duration(Duration::from_millis(cooldown_adapt));
            }

            println!("End work ({})", uploader_index);
            uploader.cooldown.end_work(ended_at);

            return Ok(result);
        }
    }
}

impl<S: AddSignaler<Range<u64>>, R: Read> UploaderMaxSize for UploadPool<S, R> {
    fn get_max_size(&self) -> u64 {
        self.uploaders.access()
            .iter()
            .map(|x| x.borrow().max_size)
            .min()
            .unwrap_or(0)
    }
}

impl<S: AddSignaler<Range<u64>>, R: Read> Uploader<String, R, S> for UploadPool<S, R> {
    fn do_upload(&mut self, reader: R, size: u64, signal: ProgressSignal<S>) -> Result<UploaderCoolDownResponse<String>, Error> {
        self._do_upload(reader, size, signal)
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::ops::Range;
    use std::thread::JoinHandle;
    use crate::signal::progress::{ProgressSignal,ProgressSignalAccessor};
    use crate::signal::{StoredSignal, StaticSignal};
    use crate::upload::account::{AccountCredentials, AccountSubscription};
    use crate::upload::bot::BotUploader;
    use crate::upload::pool::{UploadPool};
    use crate::upload::Uploader;
    use crate::utils::safe::{SafeAccessor};

    #[test]
    pub fn test() {
        let bot1 = BotUploader::new(AccountCredentials {
            channel_id: 0,
            access_token: "//".into(),
            subscription: AccountSubscription::Free,
        });

        let bot2 = BotUploader::new(AccountCredentials {
            channel_id: 0,
            access_token: "//".into(),
            subscription: AccountSubscription::Free,
        });

        let mut pool = UploadPool::new();

        pool.add_uploader(bot1);
        //pool.add_uploader(bot2);

        let files = vec![
            "C:\\Users\\marti\\Downloads\\SHITPOST\\FyGEeXCWYAQJQCs.jpg",
            "C:\\Users\\marti\\Downloads\\SHITPOST\\20221126_170921.jpg",
            "C:\\Users\\marti\\Downloads\\SHITPOST\\f12b7918d66c91ad115c3748547b1269.jpg",
            "C:\\Users\\marti\\Downloads\\SHITPOST\\IMG_0656.png",
            "C:\\Users\\marti\\Downloads\\we_live_we_love_we_lie_A2fAooXRmq8.webm",
            "C:\\Users\\marti\\Downloads\\ayezlaref_2023-09-11-11-36-49_1694425009476.mp4",
        ];


        let signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();

        let mut join : Vec<JoinHandle<()>> = Vec::new();
        let mut offset = 0;
        for file in files {
            let file = File::open(file).unwrap();
            let len = file.metadata().unwrap().len();
            let mut p = pool.clone();
            let signal = signal.clone_with_offset(offset);
            offset += len;
            join.push(
                std::thread::spawn(move || {
                    let result = p.do_upload(file, len, signal).unwrap();
                    println!("Uploaded | result = {:?}", result.unwrap());
                })
            );
        }

        for j in join {
            j.join().unwrap();
        }

        let mut signal = signal.get_progression().access();
        signal.retrim_ranges();
        println!("Uploaded | signal = {:?}", signal.get_signal_data());
    }
}
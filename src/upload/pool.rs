use std::{
    io::{Error, Read},
    ops::Range,
    thread::sleep,
    time::Duration,
    cell::RefCell,
};

use crate::{
    upload::{
        Uploader,
        UploaderMaxSize,
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

#[derive(Clone)]
pub struct PooledUploader<S: AddSignaler<Range<u64>>, R: Read> {
    uploader: Box<dyn Uploader<String, R, S>>,
    cooldown: CoolDown,
    max_size: u64,

    _phantom: std::marker::PhantomData<(R, S)>,
}

impl<S: AddSignaler<Range<u64>>, R: Read> PooledUploader<S, R> {
    pub fn new<U: Uploader<String, R, S> + CoolDownMs + 'static>(uploader: U) -> Self {
        Self {
            cooldown: uploader.create_cooldown_wait(),
            max_size: uploader.get_max_size(),
            uploader: Box::new(uploader),

            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct UploadPool<S: AddSignaler<Range<u64>>, R: Read> {
    uploaders: Safe<Vec<RefCell<PooledUploader<S, R>>>>,
}

impl<S: AddSignaler<Range<u64>>, R: Read> UploadPool<S, R> {
    fn next_uploader(&self, uploaders: &Vec<RefCell<PooledUploader<S, R>>>) -> usize {
        // TO THINK: take other data for the computation of the best uploader
        // like X-RateLimit-Remaining for bot requests
        uploaders.iter()
            .enumerate()
            .min_by_key(|(_, x)| {
            let cell = x.borrow();
            if cell.cooldown.is_working() {
                u64::MAX
            } else {
                cell.cooldown.remaining_wait()
            }
        }).map(|(i, _)| i).unwrap()
    }

    fn _do_upload(
        &self,
        reader: R,
        size: u64,
        signal: ProgressSignal<S>,
    ) -> Result<String, Error> {
        // acquire lock on uploaders
        let uploaders = self.uploaders.access();
        loop {
            // find next best uploader
            let uploader_index = self.next_uploader(&uploaders);

            let mut uploader = uploaders.get(uploader_index)
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "No uploader available"))?
                .borrow_mut();

            // if cooldown is currently regenerating
            // wait 50ms and try again
            if uploader.cooldown.is_working() {
                sleep(Duration::from_millis(50));
                continue;
            }

            uploader.cooldown.wait(); // wait for cooldown

            // mark uploader as working
            uploader.cooldown.start_work();

            // clone uploader
            let mut upl = uploader.uploader.clone();

            // drop guards = allows other execution of for other threads of '_do_upload'
            drop(uploader);
            drop(uploaders);

            let result = upl.do_upload(reader, size, signal);

            // acquire lock on uploaders
            let uploaders = self.uploaders.access();
            let mut uploader = uploaders.get(uploader_index)
                .ok_or_else(|| Error::new(std::io::ErrorKind::Other, "An error occured: cannot unlock uploader"))?
                .borrow_mut();

            uploader.cooldown.end_work();

            return result;
        }
    }
}
impl<S: AddSignaler<Range<u64>> + Clone, R: Read + Clone> UploaderMaxSize for UploadPool<S, R> {
    fn get_max_size(&self) -> u64 {
        self.uploaders.access()
            .iter()
            .map(|x| x.borrow().max_size)
            .min()
            .unwrap_or(0)
    }
}

impl<S: AddSignaler<Range<u64>> + Clone, R: Read + Clone> Uploader<String, R, S> for UploadPool<S, R> {
    fn do_upload(&mut self, reader: R, size: u64, signal: ProgressSignal<S>) -> Result<String, Error> {
        self._do_upload(reader, size, signal)
    }
}
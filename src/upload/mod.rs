pub mod account;
pub mod pool;

use std::{
    io::{Read, Error},
    ops::Range,
};
use dyn_clone::DynClone;
use crate::{
    signal::{
        AddSignaler,
        progress::{
            ProgressSignal
        },
    }
};

pub trait UploaderMaxSize {
    fn get_max_size(&self) -> u64;
}

pub trait Uploader<T, R: Read, S: AddSignaler<Range<u64>>>: DynClone + UploaderMaxSize {
    fn do_upload(&mut self, reader: R, size: u64, signal: ProgressSignal<S>) -> Result<T, Error>;
}

dyn_clone::clone_trait_object!(<T, R, S> Uploader<T, R, S> where R: Read, S: AddSignaler<Range<u64>>);
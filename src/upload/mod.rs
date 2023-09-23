pub mod account;
pub mod pool;
pub mod bot;
pub mod container;

use std::{
    io::{Read, Error},
    ops::Range,
};
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

pub enum UploaderCoolDownResponse<T> {
    CoolDown(T, u64, u32),
    Success(T),
}

impl<T> UploaderCoolDownResponse<T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::CoolDown(t, _,_) => t,
            Self::Success(t) => t,
        }
    }
}

pub trait Uploader<T, R: Read, S: AddSignaler<Range<u64>>>: UploaderMaxSize {
    fn do_upload(&mut self, reader: R, size: u64, signal: &mut ProgressSignal<S>) -> Result<UploaderCoolDownResponse<T>, Error>;
}

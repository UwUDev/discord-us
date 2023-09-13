use std::ops::Sub;
use std::cmp::{PartialOrd};

pub mod uploader;
pub mod downloader;
mod http_client;
pub mod common;
pub mod signal;
pub mod upload;
pub mod _signal;

pub mod pack;
pub mod fs;
pub mod utils;

/// Trait for getting size of something
pub trait Size {
    fn get_size(&self) -> u64;
}

impl<T: Size> Size for [T] {
    fn get_size(&self) -> u64 {
        self.iter().map(|x| x.get_size()).sum()
    }
}


pub trait ZeroSubstract  {
    /// Substract two numbers
    /// If the substraction is negative, return 0
    ///
    /// * `other` - The other number to substract
    fn zero_substract(self, other: Self) -> Self;
}

impl<T: Sub<Output = T> + PartialOrd + Default> ZeroSubstract for T {
    fn zero_substract(self, other: Self) -> Self {
        if other > self {
            Self::default()
        } else {
            self - other
        }
    }
}
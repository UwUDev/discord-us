pub mod crypt;
pub mod key;
pub mod container;

use std::ops::{Range, Sub};

pub trait Size<A> {
    fn get_size(&self) -> A;
}

impl<A: Sub<Output=A> + Copy> Size<A> for Range<A> {
    fn get_size(&self) -> A {
        self.end - self.start
    }
}
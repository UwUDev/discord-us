use std::ops::{Range};
use crate::Size;

pub trait Ranged {
    fn get_range(&self) -> Range<u64>;
}

pub trait RangedSort {
    fn sort_ranges(&mut self);
}

impl<R: Ranged> RangedSort for [R] {
    fn sort_ranges(&mut self) {
        self.sort_by(|a, b| a.get_range().start.cmp(&b.get_range().start));
    }
}

impl<T: Ranged> Size for T {
    fn get_size(&self) -> u64 {
        let range = self.get_range();
        range.end - range.start
    }
}

pub trait Intersect {
    fn is_intersecting(&self, other: &Self) -> bool;
}

impl Intersect for Range<u64> {
    fn is_intersecting(&self, other: &Self) -> bool {
        self.start <= other.end && (other.start <= self.end || other.end <= self.end)
    }
}
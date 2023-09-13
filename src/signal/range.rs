use crate::{
    signal::{AddSignaler, StoredSignal},
    utils::range::{RangedSort},
};

use std::{
    ops::{Range}
};

impl AddSignaler<Range<u64>> for StoredSignal<Vec<Range<u64>>> {
    fn add_signal(&mut self, t: Range<u64>) {
        self.data.push(t);
        self.callback_manager.run_callback(&self.data);
    }
}

impl Default for StoredSignal<Vec<Range<u64>>> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl StoredSignal<Vec<Range<u64>>> {
    /// Retrim the ranges
    /// If there are two ranges that are next to each other, merge them
    /// store the result in signal's data
    pub fn retrim_ranges(&mut self) {
        let mut ranges = self.data.clone();

        if ranges.len() == 0 {
            return;
        }

        ranges.sort_ranges();

        let mut new_ranges = Vec::new();

        let mut current_range = ranges[0].clone();

        for range in ranges.iter() {
            if range.start <= current_range.end {
                current_range.end = range.end;
            } else {
                new_ranges.push(current_range);
                current_range = range.clone();
            }
        }

        new_ranges.push(current_range.clone());

        self.data = new_ranges;
    }
}

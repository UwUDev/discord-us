use crate::{
    signal::{AddSignaler, StoredSignal},
    utils::range::{RangedSort},
};

use std::{
    ops::{Range}
};
use crate::signal::DerivedSignal;

impl AddSignaler<Range<u64>> for StoredSignal<Vec<Range<u64>>> {
    fn add_signal(&mut self, t: Range<u64>) {
        self.data.push(t);
        self.callback_manager.run_callback(&self.data);
    }
}

impl<T: Default> Default for StoredSignal<T> {
    fn default() -> Self {
        Self::new(T::default())
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

impl<T: AddSignaler<Range<u64>>> AddSignaler<Range<u64>> for DerivedSignal<T, u64> {
    fn add_signal(&mut self, t: Range<u64>) {
        self.signal.borrow_mut().add_signal(Range { start: self.data + t.start, end: self.data + t.end });
    }
}

#[cfg(test)]
mod test {
    use std::{ops::Range,rc::Rc, cell::RefCell};
    use crate::signal::{AddSignaler, DerivedSignal, StoredSignal, StaticSignal};

    #[test]
    pub fn test_range() {
        let mut range: StoredSignal<Vec<Range<u64>>> = StoredSignal::default();

        let mut rc = Rc::new(RefCell::new(range));

        rc.clone().borrow_mut().add_signal(Range { start: 0, end: 10 });

        let mut r = DerivedSignal::new(10 as u64, rc.clone());

        r.add_signal(Range { start: 0, end: 10 });

        println!("{:?}", rc.borrow().get_signal_data());
    }
}
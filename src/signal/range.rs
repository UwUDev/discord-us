use crate::{
    signal::{AddSignaler, StoredSignal,DerivedSignal},
    utils::{range::{RangedSort}, safe::{Safe}},
};

use std::{
    ops::{Range},
    cell::RefCell,
    rc::Rc
};
use crate::utils::safe::SafeAccessor;

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
            if range.start == range.end {
                continue;
            }

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

impl<T> DerivedSignal<T, u64>  {
    fn convert_range(&self, t: &Range<u64>) -> Range<u64> {
        Range { start: t.start + self.data, end: t.end + self.data }
    }
}

impl<S: AddSignaler<Range<u64>>> AddSignaler<Range<u64>> for DerivedSignal<Rc<RefCell<S>>, u64> {
    fn add_signal(&mut self, t: Range<u64>) {
        self.signal.borrow_mut().add_signal(self.convert_range(&t));
    }
}

impl<S: AddSignaler<Range<u64>>> AddSignaler<Range<u64>> for DerivedSignal<Safe<S>, u64> {
    fn add_signal(&mut self, t: Range<u64>) {
        self.signal.access().add_signal(self.convert_range(&t));
    }
}

#[cfg(test)]
mod test {
    use std::{ops::Range, rc::Rc, cell::RefCell};
    use crate::signal::{AddSignaler, DerivedSignal, StoredSignal, StaticSignal, GetSignal};

    #[test]
    pub fn test_range() {
        let range: StoredSignal<Vec<Range<u64>>> = StoredSignal::default();

        let rc = Rc::new(RefCell::new(range));

        rc.clone().borrow_mut().add_signal(Range { start: 0, end: 10 });

        let mut r = DerivedSignal::new(10 as u64, rc.clone());

        r.add_signal(Range { start: 0, end: 10 });

        println!("{:?}", rc.borrow_mut().get_signal().get_signal_data());
    }
}
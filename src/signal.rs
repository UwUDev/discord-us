use core::cmp::{Ord, PartialOrd, PartialEq};
use std::cmp::Ordering;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use sorted_vec::SortedVec;


pub trait Signal<D, R>
    where
        D: Sized,
        R: Sized, {
    fn report_data(&mut self, t: R);

    fn get_data(&self) -> D;
}

// Linear progression
// Only meant for 1 updater
// multiple readers
pub struct LinearProgression {
    progression: Arc<AtomicU64>,
}

impl LinearProgression {
    pub fn new() -> Self {
        Self {
            progression: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Clone for LinearProgression {
    fn clone(&self) -> Self {
        Self {
            progression: self.progression.clone(),
        }
    }
}

impl Signal<u64, u64> for LinearProgression {
    fn report_data(&mut self, t: u64) {
        let atomic = &self.progression;

        atomic.fetch_add(t, std::sync::atomic::Ordering::SeqCst);
    }

    fn get_data(&self) -> u64 {
        let atomic = &self.progression;

        atomic.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Clone, Debug)]
pub struct ProgressionRange {
    range_start: u64,
    range_end: u64,
}

impl PartialEq for ProgressionRange {
    fn eq(&self, other: &Self) -> bool {
        self.range_start == other.range_start && self.range_end == other.range_end
    }
}

impl PartialOrd for ProgressionRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.range_start < other.range_start {
            Some(Ordering::Less)
        } else if self.range_start > other.range_start {
            Some(Ordering::Greater)
        } else if self.range_end < other.range_end {
            Some(Ordering::Less)
        } else if self.range_end > other.range_end {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl Eq for ProgressionRange {}

impl Ord for ProgressionRange {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.range_start < other.range_start {
            Ordering::Less
        } else if self.range_start > other.range_start {
            Ordering::Greater
        } else if self.range_end < other.range_end {
            Ordering::Less
        } else if self.range_end > other.range_end {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

#[derive(Clone, Debug)]
pub struct PartProgression {
    progression: Arc<Mutex<SortedVec<ProgressionRange>>>,
}

impl ProgressionRange {
    pub fn of(range_start: u64, range_end: u64) -> Self {
        Self {
            range_start,
            range_end,
        }
    }
}

impl PartProgression {
    pub fn new() -> Self {
        Self {
            progression: Arc::new(Mutex::new(SortedVec::new())),
        }
    }
}

impl Signal<Vec<ProgressionRange>, ProgressionRange> for PartProgression {
    fn report_data(&mut self, t: ProgressionRange) {
        let mut ranges = self.progression.lock().unwrap();

        ranges.push(t);

        // not optimal but i can't use retain_mut on SortedVec
        // because it does not implement DerefMut.
        // the solution is to clone into a Vec and then replace the
        // current SortedVec with the new Vec

        let mut vec = ranges.to_vec();

        let mut prev_i = 0;

        for i in 0..vec.len() {
            let current = vec.get(i).unwrap().clone();

            if prev_i != i {
                let prev = vec.get_mut(prev_i).unwrap();

                if current.range_start <= prev.range_end {
                    prev.range_end = current.range_end;
                    vec.swap_remove(i);
                } else {
                    prev_i = i;
                }
            }
        }

        *ranges = SortedVec::from_unsorted(vec);
    }

    fn get_data(&self) -> Vec<ProgressionRange> {
        self.progression.lock().unwrap().to_vec()
    }
}

// Test
#[cfg(test)]
mod tests {
    use crate::signal::{PartProgression, Signal, ProgressionRange};

    #[test]
    fn test_ranges() {
        let mut signal = PartProgression::new();

        signal.report_data(ProgressionRange::of(0, 10));
        signal.report_data(ProgressionRange::of(10, 20));

        signal.report_data(ProgressionRange::of(30, 40));
        signal.report_data(ProgressionRange::of(29, 40));

        signal.report_data(ProgressionRange::of(50, 60));
        signal.report_data(ProgressionRange::of(51, 69));

        println!("{:?}", signal.get_data());
    }
}

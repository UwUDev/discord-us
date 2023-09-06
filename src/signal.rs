use core::cmp::{Ord, PartialOrd, PartialEq};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Sub};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use sorted_vec::SortedVec;
use dyn_clone::{DynClone, clone_trait_object};
use pbkdf2::password_hash::Output;

pub trait ReportSignal<D>: DynClone
    where D: Sized {
    fn report_data(&mut self, t: D);
}

clone_trait_object!(<D> ReportSignal<D> where D: Sized + 'static);

pub trait Signal<D>
    where
        D: Sized {
    fn get_data(&self) -> D;
}

// Linear progression
// Only meant for 1 updater
// multiple readers
#[derive(Clone, Debug)]
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

impl ReportSignal<u64> for LinearProgression {
    fn report_data(&mut self, t: u64) {
        let atomic = &self.progression;

        atomic.fetch_add(t, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Signal<u64> for LinearProgression {
    fn get_data(&self) -> u64 {
        let atomic = &self.progression;

        atomic.load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub trait Integer: Add<Output=Self> + AddAssign + Sub<Output=Self> + Ord + PartialOrd + PartialEq + Clone + Copy + Debug + Zero {}

pub trait Zero {
    fn zero() -> Self;
}

//impl<T> Integer for T where T: Add + AddAssign + Ord + PartialOrd + PartialEq + Clone + Debug + 'static {}

impl Integer for u64 {}

impl Zero for u64 {
    fn zero() -> Self {
        0
    }
}


#[derive(Clone, Debug)]
pub struct ProgressionRange<T>
    where
        T: Integer {
    pub range_start: T,
    pub range_end: T,
}

impl<T: Integer> PartialEq for ProgressionRange<T> {
    fn eq(&self, other: &Self) -> bool {
        self.range_start == other.range_start && self.range_end == other.range_end
    }
}

impl<T: Integer> PartialOrd for ProgressionRange<T> {
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

impl<T: Integer> Eq for ProgressionRange<T> {}

impl<T: Integer> Ord for ProgressionRange<T> {
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
pub struct PartProgression<T>
    where T: Integer {
    progression: Arc<Mutex<SortedVec<ProgressionRange<T>>>>,
}

impl<T: Integer> ProgressionRange<T> {
    pub fn of(range_start: T, range_end: T) -> Self {
        Self {
            range_start,
            range_end,
        }
    }
}

impl<T: Integer> PartProgression<T> {
    pub fn new() -> Self {
        Self {
            progression: Arc::new(Mutex::new(SortedVec::new())),
        }
    }

    pub fn retrim_ranges(&mut self) {
        // not optimal but i can't use retain_mut on SortedVec
        // because it does not implement DerefMut.
        // the solution is to clone into a Vec and then replace the
        // current SortedVec with the new Vec

        let mut ranges = self.progression.lock().unwrap();

        let mut vec = ranges.to_vec();

        let mut prev_i = 0;

        let mut new_vec: Vec<ProgressionRange<T>> = Vec::new();

        for i in 0..vec.len() {
            let current = vec.get(i).unwrap().clone();

            if prev_i != i {
                let prev = new_vec.get_mut(prev_i).unwrap();

                if current.range_start <= prev.range_end {
                    //println!("Merged {:?}->{:?} + {:?}->{:?} = {:?}->{:?}", prev.range_start, prev.range_end, current.range_start, current.range_end, prev.range_start, current.range_end);
                    prev.range_end = current.range_end;
                    continue;
                }
            }

            prev_i = new_vec.len();
            new_vec.push(current);
        }

        //println!("Did retrim on size {}", new_vec.len());

        *ranges = SortedVec::from(new_vec);
    }

    pub fn get_total(&self) -> T {
        let mut total: T = T::zero();
        for range in self.get_data().iter() {
            total += range.range_end - range.range_start;
        }
        total
    }
}

impl<T: Integer + Clone> ReportSignal<ProgressionRange<T>> for PartProgression<T> {
    fn report_data(&mut self, t: ProgressionRange<T>) {
        self.progression.lock().unwrap().push(t);
    }
}

impl<T: Integer> Signal<Vec<ProgressionRange<T>>> for PartProgression<T> {
    fn get_data(&self) -> Vec<ProgressionRange<T>> {
        self.progression.lock().unwrap().to_vec()
    }
}

#[derive(Clone)]
pub struct LinearPartSignal<T>
    where T: Integer + 'static {
    signal: Box<dyn ReportSignal<ProgressionRange<T>>>,
    cursor: T,
}

impl<T: Integer + Clone> ReportSignal<T> for LinearPartSignal<T> {
    fn report_data(&mut self, t: T) {
        self.signal.report_data(ProgressionRange::of(self.cursor, self.cursor + t));

        self.cursor += t;
    }
}

impl<T: Integer> LinearPartSignal<T> {
    pub fn new(signal: Box<dyn ReportSignal<ProgressionRange<T>>>, cursor: T) -> Self {
        Self { signal, cursor }
    }
}

// impl<U> ReportSignal<U> for EmptySignal<U> {
//     fn report_data(&mut self, t: U) {}
// }

// Test
#[cfg(test)]
mod tests {
    use crate::signal::{PartProgression, Signal, ProgressionRange, ReportSignal, LinearPartSignal};

    #[test]
    fn test_ranges() {
        let mut signal = PartProgression::new();

        signal.report_data(ProgressionRange::of(0, 10));
        signal.report_data(ProgressionRange::of(10, 20));

        signal.report_data(ProgressionRange::of(100, 110));

        signal.report_data(ProgressionRange::of(20, 30));
        signal.report_data(ProgressionRange::of(30, 40));

        signal.report_data(ProgressionRange::of(110, 120));

        signal.report_data(ProgressionRange::of(40, 50));
        signal.report_data(ProgressionRange::of(50, 60));

        signal.report_data(ProgressionRange::of(120, 130));

        signal.retrim_ranges();
        println!("{:?}", signal.get_data());

        // let mut s = LinearPartSignal::new(Box::new(signal.clone()), 71 as u64);
        //
        // s.report_data(10);
        // s.report_data(40);
        //
        //
        // println!("{:?}", signal.get_data());
    }
}

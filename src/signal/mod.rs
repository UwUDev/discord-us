pub mod range;

use std::{
    ops::{
        Add,
        Range,
    }
};

use crate::{
    utils::{
        range::{RangedSort, Ranged}
    }
};

/// Static signal is a signal whose data can be accessed whenever you want
/// It can be accessed by calling get_signal_data
pub trait StaticSignal<T> {
    /// Get the data of the signal
    fn get_signal_data(&self) -> &T;
}

/// Dynamic signal is a signal that can be listened to
/// It can be listened to by adding a callback
pub trait DynamicSignal<T, F: Fn(&T) + Send + 'static> {
    /// Add a callback when the signal change
    /// Return the id of the callback
    fn on_signal(&mut self, f: F) -> u32;
}

/// Signaler is a signal that can emit a signal
pub trait Signaler<T> {
    /// Emit a signal
    fn signal(&mut self, t: T);
}

/// AddSignaler is a signal that can emit a signal and add a value to the signal
pub trait AddSignaler<T> {
    /// Add a value to the signal
    /// The value is added to the current value of the signal
    fn add_signal(&mut self, t: T);
}

/// Callback

struct Callback<T> {
    callback: Box<dyn Fn(&T) + Send + 'static>,
    id: u32,
}

pub struct CallbackManager<T> {
    callbacks: Vec<Callback<T>>,
    current_id: u32,
}

impl<T> CallbackManager<T> {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
            current_id: 0,
        }
    }

    pub fn add_callback<F: Fn(&T) + Send + 'static>(&mut self, f: F) -> u32 {
        let id = self.increment_id();

        self.callbacks.push(Callback {
            callback: Box::new(f),
            id,
        });

        id
    }

    pub fn remove_callback(&mut self, id: u32) {
        self.callbacks.retain(|x| x.id != id);
    }

    pub fn increment_id(&mut self) -> u32 {
        let id = self.current_id;
        self.current_id += 1;
        id
    }

    pub fn run_callback(&self, t: &T) {
        for callback in self.callbacks.iter() {
            (callback.callback)(t);
        }
    }
}

impl<T> Signaler<T> for CallbackManager<T> {
    fn signal(&mut self, t: T) {
        self.run_callback(&t);
    }
}

impl<T, F: Fn(&T) + Send + 'static> DynamicSignal<T, F> for CallbackManager<T> {
    fn on_signal(&mut self, f: F) -> u32 {
        self.add_callback(f)
    }
}

impl<T> Default for CallbackManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Stored signal
/// A signal that can be listened to and whose data can be accessed
/// It is a combination of StaticSignal and DynamicSignal
struct StoredSignal<T> {
    data: T,

    callback_manager: CallbackManager<T>,
}

impl<T> StoredSignal<T> {
    /// Create a new StoredSignal
    ///
    /// * `data` - The data of the signal
    fn new(data: T) -> Self {
        Self {
            data,
            callback_manager: Default::default(),
        }
    }
}

impl<T> StaticSignal<T> for StoredSignal<T> {
    fn get_signal_data(&self) -> &T {
        &self.data
    }
}

impl<T> Signaler<T> for StoredSignal<T> {
    fn signal(&mut self, t: T) {
        self.data = t;
        self.callback_manager.run_callback(&self.data);
    }
}

impl<T, F: Fn(&T) + Send + 'static> DynamicSignal<T, F> for StoredSignal<T> {
    fn on_signal(&mut self, f: F) -> u32 {
        self.callback_manager.add_callback(f)
    }
}

trait AddSignalerDefault<T>: StaticSignal<T> + Signaler<T> {}

impl<T: Add<Output=T> + Copy, S: AddSignalerDefault<T>> AddSignaler<T> for S {
    fn add_signal(&mut self, t: T) {
        let value = self.get_signal_data();

        self.signal(*value + t);
    }
}

impl AddSignaler<Range<u64>> for StoredSignal<Vec<Range<u64>>> {
    fn add_signal(&mut self, t: Range<u64>) {
        self.data.push(t);
        self.callback_manager.run_callback(&self.data);
    }
}

impl StoredSignal<Vec<Range<u64>>> {
    /// Retrim the ranges
    /// If there are two ranges that are next to each other, merge them
    pub fn retrim_ranges(&mut self) {
        let mut ranges = self.data.clone();

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

#[cfg(test)]
mod test {
    use std::ops::Range;
    use crate::signal::{DynamicSignal, StoredSignal, Signaler, StaticSignal, AddSignaler};

    #[test]
    pub fn test_s() {
        let mut signal: StoredSignal<Vec<Range<u64>>> = StoredSignal::new(Vec::new());

        signal.on_signal(|x| {
            println!("Signal: {:?}", x);
        });

        signal.add_signal(Range { start: 0, end: 10 });
        signal.add_signal(Range { start: 5, end: 15 });
        signal.add_signal(Range { start: 15, end: 30 });
        signal.add_signal(Range { start: 50, end: 65 });

        signal.retrim_ranges();

        println!("Signal (final) : {:?}", signal.get_signal_data());
    }
}

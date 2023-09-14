pub mod range;
pub mod bool;
pub mod progress;

use std::{
    ops::{
        Add,
    },
    rc::{
        Rc,
    },
    cell::{
        RefCell,
    },
};

/// Static signal is a signal whose data can be accessed whenever you want
/// It can be accessed by calling get_signal_data
pub trait StaticSignal<T> {
    /// Get the data of the signal
    fn get_signal_data(&self) -> &T;
}

pub trait SignalValue<T> {
    fn get_value(&self) -> T;
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
pub struct StoredSignal<T> {
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

impl<T: Clone> SignalValue<T> for StoredSignal<T> {
    fn get_value(&self) -> T {
        self.data.clone()
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

impl<T: Add<Output=T> + Copy, S: StaticSignal<T> + Signaler<T>> AddSignaler<T> for S {
    fn add_signal(&mut self, t: T) {
        let value = self.get_signal_data();

        self.signal(*value + t);
    }
}

pub struct DerivedSignal<T, D> {
    data: D,
    signal: Rc<RefCell<T>>,
}

impl <T, D> DerivedSignal<T, D>  {
    pub fn new(data: D, signal: Rc<RefCell<T>>) -> Self {
        Self {
            data,
            signal,
        }
    }
}

#[cfg(test)]
mod test {
    use std::{ops::Range, thread::spawn};
    use crate::signal::{DynamicSignal, StoredSignal, Signaler, StaticSignal, AddSignaler};
    use crate::utils::safe::{Safe, SafeAccessor};

    #[test]
    pub fn test_s() {
        let signal: StoredSignal<Vec<Range<u64>>> = StoredSignal::new(Vec::new());

        let signal = Safe::wrap(signal);

        signal.access().on_signal(|x| {
            println!("Signal: {:?}", x);
        });

        let s = signal.clone();
        let s2 = signal.clone();

        spawn(move || {
            s2.access().signal(vec![Range { start: 100, end: 110 }]);
        });

        let j = spawn(move || {
            s.access().add_signal(Range { start: 0, end: 10 });
            s.access().add_signal(Range { start: 0, end: 20 });
            s.access().add_signal(Range { start: 0, end: 30 });
            s.access().add_signal(Range { start: 0, end: 40 });
        });


        signal.access().retrim_ranges();

        println!("Signal (final) : {:?}", signal.access().get_signal_data());

        j.join().unwrap();

        signal.access().retrim_ranges();

        println!("Signal (final2) : {:?}", signal.access().get_signal_data());
    }
}

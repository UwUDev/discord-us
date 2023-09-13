use std::{
    sync::{Arc, Mutex, MutexGuard}
};

/// A thread safe guard
pub struct Safe<T: ?Sized> {
    guarded: Arc<Mutex<T>>,
}

impl<T> Safe<T> {
    /// Wrap a value into a thread safe
    ///
    /// * `data` - The value to wrap
    pub fn wrap(data: T) -> Self {
        Self {
            guarded: Arc::new(Mutex::new(data))
        }
    }

    /// Access the value
    pub fn access<'a>(&'a self) -> MutexGuard<'a, T> {
        self.guarded.lock().unwrap()
    }
}

impl<T> Clone for Safe<T> {
    /// Clone the signal and keep the reference to it
    fn clone(&self) -> Self {
        Self {
            guarded: self.guarded.clone()
        }
    }
}
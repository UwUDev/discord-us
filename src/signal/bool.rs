use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::signal::{CallbackManager, DynamicSignal, Signaler, SignalValue};
use crate::utils::safe::{Safe, SafeAccessor};

pub struct SafeBoolSignal {
    value: Arc<AtomicBool>,
    callback_manager: Safe<CallbackManager<bool>>,
}

impl Clone for SafeBoolSignal {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            callback_manager: self.callback_manager.clone(),
        }
    }
}

impl SafeBoolSignal {
    /// Create a new thread safe StoredSignal containing a boolean
    ///
    /// * `data` - The data of the signal
    pub fn new(data: bool) -> Self {
        Self {
            value: Arc::new(AtomicBool::new(data)),
            callback_manager: Safe::wrap(Default::default()),
        }
    }
}


impl Signaler<bool> for SafeBoolSignal {
    fn signal(&mut self, t: bool) {
        self.value.store(t, std::sync::atomic::Ordering::Relaxed);
        self.callback_manager.access().run_callback(&t);
    }
}

impl SignalValue<bool> for SafeBoolSignal {
    fn get_value(&self) -> bool {
        self.value.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl<F: Fn(&bool) + Send + 'static> DynamicSignal<bool, F> for SafeBoolSignal {
    fn on_signal(&mut self, f: F) -> u32 {
        self.callback_manager.access().add_callback(f)
    }
}


use std::{ops::Range};
use crate::{
    signal::{
        bool::{SafeBoolSignal},
        StoredSignal,
        Signaler,
        SignalValue,
    },
    utils::{
        safe::{Safe}
    },
};
use crate::signal::AddSignaler;

#[derive(Clone)]
struct ProgressSignal {
    progression: Safe<StoredSignal<Vec<Range<u64>>>>,

    status: SafeBoolSignal,
}

unsafe impl Send for ProgressSignal {}

impl ProgressSignal {
    pub fn new() -> Self {
        Self {
            progression: Safe::wrap(StoredSignal::default()),
            status: SafeBoolSignal::new(true),
        }
    }

    pub fn get_progression(&self) -> &Safe<StoredSignal<Vec<Range<u64>>>> {
        &self.progression
    }

    pub fn get_status(&self) -> &SafeBoolSignal {
        &self.status
    }

    /// Send a stop signal
    pub fn stop(&mut self) {
        self.status.signal(false)
    }

    /// Check if the signal is running
    pub fn is_running(&self) -> bool {
        self.status.get_value()
    }

    /// Report progression
    pub fn report_progression(&mut self, range: Range<u64>) {
        self.progression.access().add_signal(range);
    }
}
use std::{ops::Range};
use crate::{
    signal::{
        bool::{SafeBoolSignal},
        StoredSignal,
        Signaler,
        AddSignaler,
        SignalValue,
        DerivedSignal,
    },
    utils::{
        safe::{Safe, SafeAccessor},
        range::{Ranged},
    },
};

#[derive(Clone)]
pub struct ProgressSignal<T> {
    progression: Safe<T>,

    status: SafeBoolSignal,
}

unsafe impl<T> Send for ProgressSignal<T> {}

pub trait ProgressSignalAccessor<T> {
    fn get_progression(&self) -> &Safe<T>;

    fn get_status(&self) -> &SafeBoolSignal;

    fn get_status_mut(&mut self) -> &mut SafeBoolSignal;
}

impl<T> ProgressSignalAccessor<T> for ProgressSignal<T> {
    fn get_progression(&self) -> &Safe<T> {
        &self.progression
    }

    fn get_status(&self) -> &SafeBoolSignal {
        &self.status
    }

    fn get_status_mut(&mut self) -> &mut SafeBoolSignal {
        &mut self.status
    }
}

impl<T: AddSignaler<Range<u64>>> AddSignaler<Range<u64>> for ProgressSignal<T> {
    fn add_signal(&mut self, t: Range<u64>) {
        self.get_progression().access().add_signal(t);
    }
}

pub trait ProgressSignalTrait<T, S>: ProgressSignalAccessor<T> {
    /// Send a stop signal
    fn stop(&mut self) {
        self.get_status_mut().signal(false)
    }

    /// Check if the signal is running
    fn is_running(&self) -> bool {
        self.get_status().get_value()
    }

    /// Report progression
    fn report_progression(&mut self, range: S);
}

impl<T: ProgressSignalAccessor<S>, S: AddSignaler<Range<u64>>> ProgressSignalTrait<S, Range<u64>> for T {
    fn report_progression(&mut self, range: Range<u64>) {
        self.get_progression().access().add_signal(range);
    }
}


//impl ProgressSignalTrait<StoredSignal<Vec<Range<u64>>>> for ProgressSignal {}


impl<T: Default> ProgressSignal<StoredSignal<T>> {
    pub fn new() -> ProgressSignal<StoredSignal<T>> {
        Self {
            progression: Safe::wrap(StoredSignal::default()),
            status: SafeBoolSignal::new(true),
        }
    }

    pub fn with_offset(&self, offset: u64) -> Self {
        //let signal = DerivedSignal::new(offset, self.get_progression().into());
        todo!()
    }
}
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
    },
};

pub struct ProgressSignal<T> {
    progression: Safe<T>,

    status: SafeBoolSignal,
}

unsafe impl<T> Send for ProgressSignal<T> {}

impl<T> Clone for ProgressSignal<T> {
    fn clone(&self) -> Self {
        Self {
            progression: self.progression.clone(),
            status: self.status.clone(),
        }
    }
}

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


impl<T: Default> ProgressSignal<StoredSignal<T>> {
    pub fn new() -> ProgressSignal<StoredSignal<T>> {
        Self {
            progression: Safe::wrap(StoredSignal::default()),
            status: SafeBoolSignal::new(true),
        }
    }

    pub fn clone_with_offset(&self, offset: u64) -> ProgressSignal<DerivedSignal<Safe<StoredSignal<T>>, u64>> {
        let signal: DerivedSignal<Safe<StoredSignal<T>>, u64> = DerivedSignal::new(offset, self.progression.clone());

        ProgressSignal {
            progression: Safe::wrap(signal),
            status: self.status.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::{ops::Range, thread::{spawn, JoinHandle}};
    use crate::signal::{GetSignal, progress::{ProgressSignal}, StoredSignal, DynamicSignal, AddSignaler, StaticSignal};
    use crate::utils::safe::SafeAccessor;

    #[test]
    pub fn test_progress() {
        let mut signal = ProgressSignal::<StoredSignal<Vec<Range<u64>>>>::new();


        signal.progression.access().get_signal().on_signal(|x| {
            println!("Signal: {:?}", x);
        });


        signal.add_signal(0..10);

        signal.add_signal(Range {
            start: 10,
            end: 20,
        });

        let mut j: Vec<JoinHandle<()>> = Vec::new();

        for i in 1..10 {
            let mut offset = signal.clone_with_offset(i * 100);
            j.push(spawn(move || {
                for i in 0..10 {
                    offset.add_signal(Range {
                        start: i * 10,
                        end: (i + 1) * 10,
                    });
                }
            }));
        }

        for a in j {
            a.join().unwrap();
        }

        signal.progression.access().retrim_ranges();

        println!("Signal (final) : {:?}", signal.progression.access().get_signal_data());
    }
}
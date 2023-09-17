/// This file contains rate limit tools.

use std::{
    sync::{
        Arc,
        Mutex,
        MutexGuard,
    },
    time::{
        Instant,
        Duration,
    },
    thread::{sleep},
};

use crate::{
    utils::{
        safe::{Safe, SafeAccessor},
    }
};

struct RateLimiterInner {
    tokens_per_units: f64,
    last_removal: Instant,
    units: f64,
}

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<RateLimiterInner>>,
}

pub trait RateLimiterTrait {
    /// Removes tokens from the rate limiter.
    /// If the rate limiter is unable to remove the tokens, it will block the current thread until it can.
    /// This method is thread safe.
    ///
    /// * `count` - The number of tokens to remove. This must be less than or equal to the number of tokens per second.
    fn remove_tokens(&mut self, count: f64) -> Result<(), std::io::Error>;
}

/// A rate limiter that can be used to limit the number of operations per second.
impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// * `tokens_per_units` - The number of tokens per units.
    /// * `units` - The number of units of time needed to fully regenerate the bucket (in micros).
    pub fn new(tokens_per_units: f64, units: f64) -> Self {
        let last_removal = Instant::now() /*- Duration::from_micros(1_000_000)*/;

        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner {
                tokens_per_units,
                last_removal: last_removal,

                units,
            })),
        }
    }

    pub fn tokens_per_micro(tokens: f64) -> Self {
        let mut units = 1_000_000.0;
        if tokens < 1.0 {
            units = units / tokens;
        }
        Self::new(tokens, units)
    }

    pub fn tokens_per_seconds(tokens: f64) -> Self {
        Self::tokens_per_micro(tokens / 1_000_000.0)
    }

    fn compute_sleep_time(&self, count: f64, inner: &mut MutexGuard<'_, RateLimiterInner>) -> u64 {
        let elapsed = inner.last_removal.elapsed().as_micros() as u64;

        let remaining = f64::min(inner.tokens_per_units * elapsed as f64, inner.tokens_per_units * inner.units);

        if remaining >= count {
            return 0;
        }

        let sleep_time = (count - remaining) as f64 / inner.tokens_per_units;

        return sleep_time as u64;
    }

    fn _remove_tokens(&self, inner: &mut MutexGuard<'_, RateLimiterInner>, count: f64) -> Result<(), std::io::Error> {
        //if count > (inner.tokens_per_micros * 1_000_000.0) {
        //    return Err(std::io::Error::new(std::io::ErrorKind::Other, "Cannot remove more tokens than the rate limiter allows"));
        //}

        // convert tokens into micros
        let micros = (count / inner.tokens_per_units) as u64;

        let elapsed = inner.last_removal.elapsed().as_micros() as u64;

        let remaining = f64::min(inner.tokens_per_units * elapsed as f64, inner.tokens_per_units * inner.units);

        //println!("remaining {} < count {} | elapsed {} + micros {}", remaining, count, elapsed, micros);

        if remaining < count {
            // Compute sleep time, use locked up to
            let sleep_time = (count - remaining) as f64 / inner.tokens_per_units;

            // sleep with lock being held
            //println!("Sleeping for {} micros", sleep_time as u64);

            sleep(Duration::from_micros(sleep_time as u64));

            return self._remove_tokens(inner, count);
        }

        if elapsed > inner.units as u64 {
            inner.last_removal = Instant::now() - Duration::from_micros(inner.units as u64 - micros);
        } else {
            inner.last_removal = inner.last_removal + Duration::from_micros(micros);
        }

        Ok(())
    }
}

impl RateLimiterTrait for RateLimiter {
    fn remove_tokens(&mut self, count: f64) -> Result<(), std::io::Error> {
        let mut guard = self.inner.lock().unwrap();
        self._remove_tokens(&mut guard, count)
    }
}

pub struct VoidRateLimiter {}

impl RateLimiterTrait for VoidRateLimiter {
    fn remove_tokens(&mut self, _count: f64) -> Result<(), std::io::Error> {
        Ok(())
    }
}

/// A cooldown that can be used to wait for a certain amount of time
/// before each operation.
#[derive(Clone)]
pub enum CoolDown {
    Void(),
    RateLimiter(RateLimiter),
    Work(Safe<WorkCoolDown>),
}

#[derive(Clone)]
pub struct WorkCoolDown {
    ended: Instant,
    pub duration: Duration,
    pub is_working: bool,
}

impl CoolDown {
    /// Create a new cooldown.
    ///
    /// * `cool_down_ms` - The number of milliseconds to wait between each operation.
    pub fn new(cool_down_ms: f64) -> Self {
        Self::RateLimiter(RateLimiter::new(1.0, cool_down_ms * 1_000.0))
    }

    pub fn work(cool_down_ms: f64) -> Self {
        let duration = Duration::from_millis(cool_down_ms as u64);
        Self::Work(Safe::wrap(WorkCoolDown {
            ended: Instant::now() - duration,
            duration: duration,
            is_working: false,
        }))
    }

    pub fn wait(&mut self) {
        match self {
            CoolDown::RateLimiter(rate_limiter) => rate_limiter.remove_tokens(1.0).unwrap(),
            _ => {
                sleep(Duration::from_millis(self.remaining_wait()))
            }
        }
    }

    pub fn start_work(&mut self) {
        match self {
            CoolDown::Work(work) => {
                let mut work = work.access();
                work.is_working = true;
            }
            _ => {}
        }
    }

    pub fn end_work(&mut self, at: Instant) {
        match self {
            CoolDown::Work(work) => {
                let mut work = work.access();
                work.is_working = false;
                work.ended = at;
            }
            _ => {}
        }
    }

    pub fn set_duration(&mut self, duration: Duration) {
        match self {
            CoolDown::Work(work) => {
                let mut work = work.access();
                work.duration = duration;
            }
            _ => {}
        }
    }

    pub fn remaining_wait(&self) -> u64 {
        match self {
            CoolDown::Void() => 0,
            CoolDown::RateLimiter(rate_limiter) => rate_limiter.compute_sleep_time(1.0, &mut rate_limiter.inner.lock().unwrap()),
            CoolDown::Work(work) => {
                let work = work.access();
                let elapsed = work.ended.elapsed().as_millis() as u64;

                if elapsed > work.duration.as_millis() as u64 {
                    return 0;
                }

                return work.duration.as_millis() as u64 - elapsed;
            }
        }
    }

    pub fn is_working(&self) -> bool {
        match self {
            CoolDown::Work(work) => {
                let work = work.access();
                return work.is_working;
            }
            _ => false,
        }
    }
}

pub trait CoolDownMs {
    /// Get the cooldown in milliseconds.
    fn get_cool_down(&self) -> f64;

    fn create_cooldown_wait(&self) -> CoolDown {
        return if self.get_cool_down() < 0.0 {
            CoolDown::Void()
        } else {
            let duration = Duration::from_millis(self.get_cool_down() as u64);
            CoolDown::Work(Safe::wrap(WorkCoolDown {
                ended: Instant::now() - duration,
                duration,
                is_working: false,
            }))
        };
    }
}

#[cfg(test)]
mod test {
    use std::time::{Instant};
    use std::thread::{JoinHandle, spawn};

    use crate::utils::{
        limit::{RateLimiter, RateLimiterTrait}
    };

    #[test]
    pub fn test_limiter() {
        let l = RateLimiter::tokens_per_seconds(2500.0);

        let now = Instant::now();
        let mut j: Vec<JoinHandle<()>> = Vec::new();

        for i in 0..10 {
            let mut l = l.clone();
            let h = spawn(move || {
                for x in 0..10 {
                    l.remove_tokens(100.0).unwrap();
                    println!("{}", (i * 10) + x);
                }
            });
            j.push(h);
        }

        for h in j {
            h.join().unwrap();
        }

        println!("Elapsed {:?}", now.elapsed());
    }
}
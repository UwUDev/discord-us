/// This file contains rate limit tools.

use std::{
    sync::{
        Arc,
        Condvar,
        Mutex,
        MutexGuard,
    },
    time::{
        Instant,
        Duration,
    },
    cmp::{min},
    thread::{sleep},
};


struct RateLimiterInner {
    tokens_per_second: u64,
    last_removal: Instant,
}

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<RateLimiterInner>>,
}

impl RateLimiter {
    pub fn new(tokens_per_second: u64) -> Self {
        let last_removal = Instant::now() /*- Duration::from_micros(1_000_000)*/;

        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner {
                tokens_per_second: tokens_per_second,
                last_removal: last_removal,
            })),
        }
    }

    pub fn remove_tokens(&mut self, count: u64) -> Result<(), std::io::Error> {
        let mut guard = self.inner.lock().unwrap();
        self._remove_tokens(&mut guard, count)
    }

    fn _remove_tokens(&self, inner: &mut MutexGuard<'_, RateLimiterInner>, count: u64) -> Result<(), std::io::Error> {
        if count > inner.tokens_per_second {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Cannot remove more tokens than the rate limiter allows"));
        }

        // convert tokens into micros
        let micros = count * 1_000_000 / inner.tokens_per_second;

        let elapsed = inner.last_removal.elapsed().as_micros() as u64;

        let remaining = min(inner.tokens_per_second * elapsed / 1_000_000, inner.tokens_per_second);

        // println!("remaining {} < count {} | elapsed {} + micros {}", remaining, count, elapsed, micros);

        if remaining < count {
            // Compute sleep time, use locked up to
            let sleep_time = (count - remaining) * 1_000_000 / inner.tokens_per_second;

            // sleep with lock being held

            sleep(Duration::from_micros(sleep_time));

            return self._remove_tokens(inner, count);
        }

        if elapsed as u64 > 1_000_000 {
            inner.last_removal = Instant::now() - Duration::from_micros(1_000_000 - micros);
        } else {
            inner.last_removal = inner.last_removal + Duration::from_micros(micros);
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::time::{Instant};
    use std::thread::{JoinHandle, spawn};

    use crate::utils::{
        limit::{RateLimiter}
    };

    #[test]
    pub fn test_limiter() {
        let l = RateLimiter::new(2500);

        let now = Instant::now();
        let mut j: Vec<JoinHandle<()>> = Vec::new();

        for i in 0..10 {
            let mut l = l.clone();
            let h = spawn(move || {
                for x in 0..10 {
                    l.remove_tokens(100).unwrap();
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
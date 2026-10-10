use std::time::{Duration, Instant};

/// One clock for every span of work that klin times, in whole milliseconds.
pub fn timed<T>(work: impl FnOnce() -> T) -> (T, u64) {
    let begun = Instant::now();
    let out = work();
    (out, millis(begun.elapsed()))
}

pub fn millis(spent: Duration) -> u64 {
    u64::try_from(spent.as_millis()).unwrap_or(u64::MAX)
}

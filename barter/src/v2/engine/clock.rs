use chrono::{DateTime, Utc};
use std::time::Instant;

pub trait EngineClock {
    type Time;

    fn update(&mut self, time: Self::Time);
    fn engine_time(&self) -> Self::Time;
}

#[derive(Debug)]
pub struct LiveClock;

impl EngineClock for LiveClock {
    type Time = DateTime<Utc>;

    fn update(&mut self, _: Self::Time) {}

    fn engine_time(&self) -> Self::Time {
        Utc::now()
    }
}

#[derive(Debug)]
pub struct BacktestClock {
    pub event_time: DateTime<Utc>,
    pub last_event_instant: Instant,
}

impl Default for BacktestClock {
    fn default() -> Self {
        Self {
            event_time: DateTime::<Utc>::MIN_UTC,
            last_event_instant: Instant::now(),
        }
    }
}

impl EngineClock for BacktestClock {
    type Time = DateTime<Utc>;

    fn update(&mut self, time: Self::Time) {
        self.event_time = time;
        self.last_event_instant = Instant::now();
    }

    fn engine_time(&self) -> Self::Time {
        let elapsed = self.last_event_instant.elapsed();
        let elapsed = chrono::Duration::from_std(elapsed)
            .expect("BacktestClock last_event_instant should not be out of range");
        self.event_time + elapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backtest_clock() {
        fn sleep(duration: std::time::Duration) {
            let start = Instant::now();
            while Instant::now() - start < duration {
                // Busy-spin to improve accuracy over std::thread::sleep
            }
        }

        const ALLOWED_DIFF_NANOSECONDS: i64 = 10_000;

        let mut clock = BacktestClock::default();

        // Expect time_now to be ~10ms
        let time_expected = DateTime::from_timestamp_millis(10).unwrap();
        clock.update(time_expected);
        let time_delta = clock
            .engine_time()
            .signed_duration_since(time_expected)
            .num_nanoseconds()
            .unwrap();
        assert!(
            time_delta <= ALLOWED_DIFF_NANOSECONDS,
            "TimeDelta: {time_delta} exceeded {ALLOWED_DIFF_NANOSECONDS}"
        );

        // Expect time_now to be ~11ms
        sleep(std::time::Duration::from_millis(1));
        let time_expected = DateTime::from_timestamp_millis(11).unwrap();
        let time_delta = clock
            .engine_time()
            .signed_duration_since(time_expected)
            .num_nanoseconds()
            .unwrap();
        assert!(
            time_delta <= ALLOWED_DIFF_NANOSECONDS,
            "TimeDelta: {time_delta} exceeded {ALLOWED_DIFF_NANOSECONDS}"
        );

        // Expect time_now to be ~19ms
        sleep(std::time::Duration::from_millis(8));
        let time_expected = DateTime::from_timestamp_millis(19).unwrap();
        let time_delta = clock
            .engine_time()
            .signed_duration_since(time_expected)
            .num_nanoseconds()
            .unwrap();
        assert!(
            time_delta <= ALLOWED_DIFF_NANOSECONDS,
            "TimeDelta: {time_delta} exceeded {ALLOWED_DIFF_NANOSECONDS}"
        );

        // Expect time_now to be ~30ms
        let time_expected = DateTime::from_timestamp_millis(30).unwrap();
        clock.update(time_expected);
        let time_delta = clock
            .engine_time()
            .signed_duration_since(time_expected)
            .num_nanoseconds()
            .unwrap();
        assert!(
            time_delta <= ALLOWED_DIFF_NANOSECONDS,
            "TimeDelta: {time_delta} exceeded {ALLOWED_DIFF_NANOSECONDS}"
        );

        // Expect time_now to be ~50ms
        sleep(std::time::Duration::from_millis(20));
        let time_expected = DateTime::from_timestamp_millis(50).unwrap();
        let time_delta = clock
            .engine_time()
            .signed_duration_since(time_expected)
            .num_nanoseconds()
            .unwrap();
        assert!(
            time_delta <= ALLOWED_DIFF_NANOSECONDS,
            "TimeDelta: {time_delta} exceeded {ALLOWED_DIFF_NANOSECONDS}"
        );
    }
}

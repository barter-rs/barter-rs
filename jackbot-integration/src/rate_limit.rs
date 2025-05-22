use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, oneshot};

/// Priority levels for rate limited operations.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Priority {
    High,
    Normal,
    Low,
}

struct Waiter {
    tx: oneshot::Sender<()>,
}

struct Inner {
    capacity: usize,
    tokens: usize,
    interval: Duration,
    last_refill: Instant,
    base_interval: Duration,
    max_interval: Duration,
    high: VecDeque<Waiter>,
    normal: VecDeque<Waiter>,
    low: VecDeque<Waiter>,
}

impl Inner {
    fn refill(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_refill) >= self.interval {
            let periods = now.duration_since(self.last_refill).as_millis() / self.interval.as_millis();
            let add_tokens = (periods as usize + 1) * self.capacity;
            self.last_refill = now;
            self.tokens = usize::min(self.tokens + add_tokens, self.capacity);
            while self.tokens > 0 {
                if let Some(waiter) = self.high.pop_front()
                    .or_else(|| self.normal.pop_front())
                    .or_else(|| self.low.pop_front())
                {
                    self.tokens -= 1;
                    let _ = waiter.tx.send(());
                } else {
                    break;
                }
            }
            if self.tokens > self.capacity {
                self.tokens = self.capacity;
            }
        }
    }
}

/// Simple token bucket rate limiter with priority queues and adaptive backoff.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<Inner>>,
}

impl RateLimiter {
    /// Construct a new [`RateLimiter`] allowing `capacity` operations every `interval`.
    pub fn new(capacity: usize, interval: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                capacity,
                tokens: capacity,
                interval,
                last_refill: Instant::now(),
                base_interval: interval,
                max_interval: interval * 16,
                high: VecDeque::new(),
                normal: VecDeque::new(),
                low: VecDeque::new(),
            })),
        }
    }

    /// Acquire a permit according to the provided priority.
    pub async fn acquire(&self, priority: Priority) {
        loop {
            let rx = {
                let mut inner = self.inner.lock().await;
                inner.refill();
                if inner.tokens > 0 {
                    inner.tokens -= 1;
                    None
                } else {
                    let (tx, rx) = oneshot::channel();
                    let waiter = Waiter { tx };
                    match priority {
                        Priority::High => inner.high.push_back(waiter),
                        Priority::Normal => inner.normal.push_back(waiter),
                        Priority::Low => inner.low.push_back(waiter),
                    }
                    Some(rx)
                }
            };
            match rx {
                None => return,
                Some(rx) => {
                    let _ = rx.await;
                }
            }
        }
    }

    /// Report a rate limit violation to trigger backoff.
    pub async fn report_violation(&self) {
        let mut inner = self.inner.lock().await;
        let next = inner.interval * 2;
        inner.interval = std::cmp::min(next, inner.max_interval);
    }

    /// Reset the current backoff to the base interval.
    pub async fn reset_backoff(&self) {
        let mut inner = self.inner.lock().await;
        inner.interval = inner.base_interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration, Instant};

    #[tokio::test]
    async fn test_rate_limit_basic() {
        let rl = RateLimiter::new(2, Duration::from_millis(50));
        rl.acquire(Priority::Normal).await;
        rl.acquire(Priority::Normal).await;
        let start = Instant::now();
        rl.acquire(Priority::Normal).await;
        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_priority_queue() {
        let rl = RateLimiter::new(1, Duration::from_millis(40));
        // consume initial token
        rl.acquire(Priority::Normal).await;
        let rl1 = rl.clone();
        let t1 = tokio::spawn(async move {
            rl1.acquire(Priority::Low).await;
            Instant::now()
        });
        sleep(Duration::from_millis(10)).await;
        let rl2 = rl.clone();
        let t2 = tokio::spawn(async move {
            rl2.acquire(Priority::High).await;
            Instant::now()
        });
        let time_high = t2.await.unwrap();
        let time_low = t1.await.unwrap();
        assert!(time_high <= time_low);
    }

    #[tokio::test]
    async fn test_adaptive_backoff() {
        let rl = RateLimiter::new(1, Duration::from_millis(30));
        rl.acquire(Priority::Normal).await;
        rl.report_violation().await; // double interval
        let start = Instant::now();
        rl.acquire(Priority::Normal).await;
        assert!(start.elapsed() >= Duration::from_millis(60));
    }
}

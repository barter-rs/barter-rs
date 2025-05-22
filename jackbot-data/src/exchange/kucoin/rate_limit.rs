use jackbot_integration::rate_limit::{Priority, RateLimiter};
use std::time::Duration;

/// Kucoin API rate limiter for REST and WebSocket usage.
#[derive(Clone)]
pub struct KucoinRateLimit {
    rest: RateLimiter,
    ws: RateLimiter,
}

impl KucoinRateLimit {
    /// Create a new [`KucoinRateLimit`] using official exchange quotas.
    ///
    /// REST: 30 requests per 3 seconds per IP.
    /// WebSocket: 100 messages per 10 seconds.
    pub fn new() -> Self {
        Self::with_params(
            30,
            Duration::from_secs(3),
            100,
            Duration::from_secs(10),
            Duration::from_millis(100),
        )
    }

    /// Create a custom [`KucoinRateLimit`] with provided quotas and jitter for testing.
    pub fn with_params(
        rest_capacity: usize,
        rest_interval: Duration,
        ws_capacity: usize,
        ws_interval: Duration,
        jitter: Duration,
    ) -> Self {
        Self {
            rest: RateLimiter::new_with_jitter(rest_capacity, rest_interval, jitter),
            ws: RateLimiter::new_with_jitter(ws_capacity, ws_interval, jitter),
        }
    }

    /// Acquire a REST permit with the specified [`Priority`].
    pub async fn acquire_rest(&self, priority: Priority) {
        self.rest.acquire(priority).await;
    }

    /// Acquire a WebSocket permit with the specified [`Priority`].
    pub async fn acquire_ws(&self, priority: Priority) {
        self.ws.acquire(priority).await;
    }

    /// Report a REST rate limit violation.
    pub async fn report_rest_violation(&self) {
        self.rest.report_violation().await;
    }

    /// Report a WebSocket rate limit violation.
    pub async fn report_ws_violation(&self) {
        self.ws.report_violation().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackbot_integration::rate_limit::Priority;
    use tokio::time::{Instant, Duration};

    #[tokio::test]
    async fn test_rest_limit_exhaustion() {
        let rl = KucoinRateLimit::with_params(1, Duration::from_millis(40), 1, Duration::from_millis(40), Duration::from_millis(0));
        rl.acquire_rest(Priority::Normal).await;
        let start = Instant::now();
        rl.acquire_rest(Priority::Normal).await;
        assert!(start.elapsed() >= Duration::from_millis(40));
    }

    #[tokio::test]
    async fn test_ws_backoff_jitter() {
        let rl = KucoinRateLimit::with_params(1, Duration::from_millis(20), 1, Duration::from_millis(20), Duration::from_millis(20));
        rl.acquire_ws(Priority::Normal).await;
        rl.report_ws_violation().await; // next interval 40-60ms
        let start = Instant::now();
        rl.acquire_ws(Priority::Normal).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(40));
        assert!(elapsed <= Duration::from_millis(60));
    }
}

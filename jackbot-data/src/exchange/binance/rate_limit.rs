use jackbot_integration::rate_limit::{Priority, RateLimiter};
use std::time::Duration;

/// Binance API rate limiter for REST and WebSocket usage.
#[derive(Clone)]
pub struct BinanceRateLimit {
    rest: RateLimiter,
    ws: RateLimiter,
}

impl BinanceRateLimit {
    /// Create a new [`BinanceRateLimit`] using placeholder quotas.
    ///
    /// REST: 1200 requests per minute.
    /// WebSocket: 5 messages per second.
    pub fn new() -> Self {
        Self::with_params(
            1200,
            Duration::from_secs(60),
            5,
            Duration::from_secs(1),
            Duration::from_millis(100),
        )
    }

    /// Create a custom [`BinanceRateLimit`] with provided quotas and jitter for testing.
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
    use tokio::time::{Duration, Instant};

    #[tokio::test]
    async fn test_rest_limit_exhaustion() {
        let rl = BinanceRateLimit::with_params(1, Duration::from_millis(40), 1, Duration::from_millis(40), Duration::from_millis(0));
        rl.acquire_rest(Priority::Normal).await;
        let start = Instant::now();
        rl.acquire_rest(Priority::Normal).await;
        assert!(start.elapsed() >= Duration::from_millis(40));
    }

    #[tokio::test]
    async fn test_ws_backoff_jitter() {
        let rl = BinanceRateLimit::with_params(1, Duration::from_millis(20), 1, Duration::from_millis(20), Duration::from_millis(20));
        rl.acquire_ws(Priority::Normal).await;
        rl.report_ws_violation().await;
        let start = Instant::now();
        rl.acquire_ws(Priority::Normal).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(40));
        assert!(elapsed <= Duration::from_millis(60));
    }
}

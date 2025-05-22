use jackbot_integration::rate_limit::{Priority, RateLimiter};
use std::time::Duration;

/// Bitget API rate limiter for REST and WebSocket usage.
#[derive(Clone)]
pub struct BitgetRateLimit {
    rest: RateLimiter,
    ws: RateLimiter,
}

impl BitgetRateLimit {
    /// Create a new [`BitgetRateLimit`] using placeholder quotas.
    pub fn new() -> Self {
        Self::with_params(
            600,
            Duration::from_secs(60),
            20,
            Duration::from_secs(1),
            Duration::from_millis(100),
        )
    }

    /// Create a custom [`BitgetRateLimit`] with provided quotas and jitter for testing.
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

    pub async fn acquire_rest(&self, priority: Priority) {
        self.rest.acquire(priority).await;
    }

    pub async fn acquire_ws(&self, priority: Priority) {
        self.ws.acquire(priority).await;
    }

    pub async fn report_rest_violation(&self) {
        self.rest.report_violation().await;
    }

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
        let rl = BitgetRateLimit::with_params(1, Duration::from_millis(40), 1, Duration::from_millis(40), Duration::from_millis(0));
        rl.acquire_rest(Priority::Normal).await;
        let start = Instant::now();
        rl.acquire_rest(Priority::Normal).await;
        assert!(start.elapsed() >= Duration::from_millis(40));
    }

    #[tokio::test]
    async fn test_ws_backoff_jitter() {
        let rl = BitgetRateLimit::with_params(1, Duration::from_millis(20), 1, Duration::from_millis(20), Duration::from_millis(20));
        rl.acquire_ws(Priority::Normal).await;
        rl.report_ws_violation().await;
        let start = Instant::now();
        rl.acquire_ws(Priority::Normal).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(40));
        assert!(elapsed <= Duration::from_millis(60));
    }
}

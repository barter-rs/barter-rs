/// Calculates backoff duration between socket reconnection attempts.
pub trait ReconnectBackoff {
    /// Returns the backoff duration to wait before the next reconnection attempt.
    fn reconnect_backoff(&mut self, reconnection_attempt: u32) -> std::time::Duration;
}

impl<F> ReconnectBackoff for F
where
    F: FnMut(u32) -> std::time::Duration,
{
    #[inline]
    fn reconnect_backoff(&mut self, reconnection_attempt: u32) -> std::time::Duration {
        self(reconnection_attempt)
    }
}

/// Default exponential backoff strategy with a maximum delay cap.
///
/// Uses 2^n + 10ms formula, capped at 2^15ms.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DefaultBackoff;

impl ReconnectBackoff for DefaultBackoff {
    fn reconnect_backoff(&mut self, reconnection_attempt: u32) -> std::time::Duration {
        match reconnection_attempt {
            0 => std::time::Duration::ZERO,
            n => std::time::Duration::from_millis(2u64.pow(n.min(15)) + 10),
        }
    }
}

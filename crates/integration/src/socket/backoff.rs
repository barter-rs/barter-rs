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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_default_backoff_zero() {
        let mut backoff = DefaultBackoff;
        assert_eq!(backoff.reconnect_backoff(0), Duration::ZERO);
    }

    #[test]
    fn test_default_backoff_first_attempt() {
        let mut backoff = DefaultBackoff;
        assert_eq!(backoff.reconnect_backoff(1), Duration::from_millis(12));
    }

    #[test]
    fn test_default_backoff_at_cap() {
        let mut backoff = DefaultBackoff;
        // 2^15 + 10 = 32768 + 10 = 32778
        assert_eq!(backoff.reconnect_backoff(15), Duration::from_millis(32778));
    }

    #[test]
    fn test_default_backoff_beyond_cap() {
        let mut backoff = DefaultBackoff;
        // n=16 should be capped at 2^15 + 10
        assert_eq!(backoff.reconnect_backoff(16), Duration::from_millis(32778));
    }

    #[test]
    fn test_default_backoff_u32_max() {
        let mut backoff = DefaultBackoff;
        assert_eq!(
            backoff.reconnect_backoff(u32::MAX),
            Duration::from_millis(32778)
        );
    }

    #[test]
    fn test_closure_backoff() {
        let mut backoff = |n: u32| Duration::from_secs(n as u64);
        assert_eq!(backoff.reconnect_backoff(5), Duration::from_secs(5));
    }
}

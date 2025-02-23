use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, format_smolstr};
use std::fmt::Debug;

/// A trait for types that represent time intervals used in financial calculations.
///
/// Implementors of this trait can represent different time periods (e.g., daily, annual) and
/// provide consistent ways to access their duration and human-readable names.
///
/// # Examples
/// ```rust
/// use barter::statistic::time::{TimeInterval, Daily, Annual252, Annual365};
///
/// // Daily TimeInterval
/// let daily = Daily;
/// assert_eq!(daily.name().as_str(), "Daily");
/// assert_eq!(daily.interval().num_days(), 1);
///
/// // Traditional markets annualised TimeInterval (252 trading days per year)
/// let annual_traditional = Annual252;
/// assert_eq!(annual_traditional.name().as_str(), "Annual(252)");
/// assert_eq!(annual_traditional.interval().num_days(), 252);
///
/// // Crypto-centric annualised TimeInterval (24/7 trading)
/// let annual_crypto = Annual365;
/// assert_eq!(annual_crypto.name().as_str(), "Annual(365)");
/// assert_eq!(annual_crypto.interval().num_days(), 365);
/// ```
pub trait TimeInterval: Debug + Copy {
    fn name(&self) -> SmolStr;
    fn interval(&self) -> TimeDelta;
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct Annual365;

impl TimeInterval for Annual365 {
    fn name(&self) -> SmolStr {
        SmolStr::new("Annual(365)")
    }

    fn interval(&self) -> TimeDelta {
        TimeDelta::days(365)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct Annual252;

impl TimeInterval for Annual252 {
    fn name(&self) -> SmolStr {
        SmolStr::new("Annual(252)")
    }

    fn interval(&self) -> TimeDelta {
        TimeDelta::days(252)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct Daily;

impl TimeInterval for Daily {
    fn name(&self) -> SmolStr {
        SmolStr::new("Daily")
    }

    fn interval(&self) -> TimeDelta {
        TimeDelta::days(1)
    }
}

impl TimeInterval for TimeDelta {
    fn name(&self) -> SmolStr {
        format_smolstr!("Duration {} (minutes)", self.num_minutes())
    }

    fn interval(&self) -> TimeDelta {
        *self
    }
}

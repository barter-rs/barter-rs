use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Barter data module specific errors.
pub mod error;

/// Live market event feed for dry-trading & live-trading.
pub mod live;

/// Historical market event feed for backtesting.
pub mod historical;

/// Generates the next `Event`. Acts as the system heartbeat.
pub trait MarketGenerator<Event> {
    /// Return the next market `Event`.
    fn next(&mut self) -> Feed<Event>;
}

/// Communicates the state of the [`Feed`] as well as the next event.
#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum Feed<Event> {
    Next(Event),
    Unhealthy,
    Finished,
}

/// Metadata detailing the [`Candle`](barter_data::subscription::candle::Candle) or
/// [`Trade`](barter_data::subscription::trade::PublicTrade) close price & it's associated
/// timestamp. Used to propagate key market information in downstream Events.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MarketMeta {
    /// Close value from the source market event.
    pub close: f64,
    /// Exchange timestamp from the source market event.
    pub time: DateTime<Utc>,
}

impl Default for MarketMeta {
    fn default() -> Self {
        Self {
            close: 100.0,
            time: Utc::now(),
        }
    }
}

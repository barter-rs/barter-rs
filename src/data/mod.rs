use barter_data::model::MarketEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Barter data module specific errors.
pub mod error;

/// Live [`MarketEvent`] feed for dry-trading & live-trading.
pub mod live;

/// Historical [`MarketEvent`] feed for backtesting.
pub mod historical;

/// Generates the latest [`MarketEvent`]. Acts as the system heartbeat.
pub trait MarketGenerator {
    /// Return the latest [`MarketEvent`].
    fn generate(&mut self) -> Feed<MarketEvent>;
}

/// Communicates the state of the [`Feed`] as well as the next event.
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum Feed<Event> {
    Next(Event),
    Unhealthy,
    Finished,
}

/// Metadata detailing the [`Candle`](barter_data::model::Candle) or
/// [`Trade`](barter_data::model::PublicTrade) close price & it's associated timestamp. Used to
/// propagate key market information in downstream Events.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct MarketMeta {
    /// Close value from the source [`MarketEvent`].
    pub close: f64,
    /// Exchange timestamp from the source [`MarketEvent`].
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

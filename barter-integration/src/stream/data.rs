use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// Generic `DataStream`.
///
/// Defines how to initialise the `DataStream`, and what the stream contains.
pub trait DataStream<Args> {
    /// Stream::Item type yielded by the stream.
    type Item;

    /// Connection error type if initialisation fails.
    type Error;

    /// Initialise the `DataStream`.
    fn init(
        args: Args,
    ) -> impl Future<Output = Result<impl Stream<Item = Self::Item> + Send, Self::Error>> + Send;
}

/// Configuration arguments for initializing a data stream.
///
/// This struct encapsulates all parameters required to initialize a data stream, including
/// the streaming mode (live or historical), subscriptions, server configuration, and timeout settings.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct DataArgs<Mode, Subs, Config> {
    /// The streaming mode (eg/ [`Live`] or [`Historical`]).
    pub mode: Mode,

    /// Subscriptions defining what data to stream.
    pub subscriptions: Subs,

    /// Configuration required for the `DataStream` source (eg/ credentials, urls, timeouts, etc.).
    pub config: Config,
}

impl<Subs, Config> DataArgs<Live, Subs, Config> {
    /// Construct [`DataArgs`] for a live data stream.
    ///
    /// # Arguments
    /// * `subscriptions` - The subscriptions defining what data to stream
    /// * `config` - Server-specific configuration for the stream connection
    pub fn live(subscriptions: Subs, config: Config) -> Self {
        Self {
            mode: Live,
            subscriptions,
            config,
        }
    }
}

impl<Subs, Config> DataArgs<Historical, Subs, Config> {
    /// Construct [`DataArgs`] for a historical data stream.
    ///
    /// # Arguments
    /// * `historical` - Time range specification for the historical data
    /// * `subscriptions` - The subscriptions defining what data to stream
    /// * `config` - Server-specific configuration for the stream connection
    pub fn historical(historical: Historical, subscriptions: Subs, config: Config) -> Self {
        Self {
            mode: historical,
            subscriptions,
            config,
        }
    }
}

/// [`DataStream`] kind, either [`Live`] or [`Historical`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum StreamKind {
    /// Real-time data stream providing events as they occur.
    Live(Live),

    /// Historical data stream replaying events from a time range.
    Historical(Historical),
}

/// Live [`DataStream`] kind.
///
/// Live `DataStream`s are real-time.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Live;

/// Historical [`DataStream`] kind.
///
/// Historical `DataStream`s replay past events within a specified time range.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Historical {
    /// Start timestamp of the historical data range.
    pub start: DateTime<Utc>,

    /// Optional end timestamp of the historical data range.
    ///
    /// If `None`, the stream continues until the present or until all available historical
    /// data is consumed.
    pub end: Option<DateTime<Utc>>,
}

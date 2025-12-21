use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};

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

/// Configuration arguments for initialising a data stream.
///
/// This struct encapsulates all parameters required to initialise a data stream, including
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_args_live() {
        let args = DataArgs::live(vec!["sub1", "sub2"], "config");
        assert_eq!(args.mode, Live);
        assert_eq!(args.subscriptions, vec!["sub1", "sub2"]);
        assert_eq!(args.config, "config");
    }

    #[test]
    fn test_data_args_historical() {
        let start = Utc::now();
        let historical = Historical {
            start,
            end: None,
        };
        let args = DataArgs::historical(historical, vec!["sub"], "cfg");
        assert_eq!(args.mode.start, start);
        assert!(args.mode.end.is_none());
        assert_eq!(args.subscriptions, vec!["sub"]);
    }
}

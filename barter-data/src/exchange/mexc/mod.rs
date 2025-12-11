//! MEXC exchange integration.
//!
//! MEXC uses Protocol Buffers for orderbook data, which requires special handling
//! with [`WebSocketProtobufParser`] instead of the default JSON parser.
//!
//! ### Important Notes
//! - MEXC sends JSON for subscription responses and ping/pong
//! - MEXC sends binary Protobuf for orderbook data
//! - L2 uses deferred snapshot fetching: WebSocket updates are buffered while the REST
//!   snapshot is fetched, then aligned using version validation per MEXC's recommended
//!   approach. See [`book::l2`] for details.

use self::{
    book::{l1::MexcOrderBooksL1Transformer, l2::MexcOrderBooksL2Transformer},
    channel::MexcChannel,
    market::MexcMarket,
    subscription::MexcSubResponse,
};
use crate::{
    ExchangeWsStream, NoInitialSnapshots,
    exchange::{Connector, PingInterval, StreamSelector, subscription::ExchangeSub},
    instrument::InstrumentData,
    subscriber::{WebSocketSubscriber, validator::WebSocketSubValidator},
    subscription::{Map, book::{OrderBooksL1, OrderBooksL2}},
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketProtobufParser, WsMessage},
};
use std::time::Duration;
use url::Url;

/// MEXC orderbook types and transformers.
pub mod book;

/// MEXC channel definitions.
pub mod channel;

/// MEXC market identifier.
pub mod market;

/// MEXC protobuf message definitions.
pub mod proto;

/// MEXC subscription response handling.
pub mod subscription;

/// MEXC WebSocket URL.
///
/// **Important**: Use `wss://wbs-api.mexc.com/ws`, NOT `wss://wbs.mexc.com/ws`
/// which blocks v3 subscriptions.
pub const MEXC_WS_URL: &str = "wss://wbs-api.mexc.com/ws";

/// Default snapshot depth for L2 orderbook.
pub const DEFAULT_SNAPSHOT_DEPTH: u32 = 500;

/// Type alias for MEXC WebSocket stream using Protobuf parser.
pub type MexcWsStream<Transformer> = ExchangeWsStream<WebSocketProtobufParser, Transformer>;

/// MEXC Spot exchange connector.
///
/// ### Configuration
/// The snapshot depth for L2 orderbooks can be configured:
/// ```ignore
/// // Use default depth (500)
/// let mexc = MexcSpot::default();
///
/// // Use custom depth
/// let mexc = MexcSpot::with_snapshot_depth(1000);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MexcSpot {
    /// Depth limit for L2 orderbook snapshots (default: 500).
    /// Valid values: 5, 10, 20, 50, 100, 500, 1000.
    pub snapshot_depth: u32,
}

impl Default for MexcSpot {
    fn default() -> Self {
        Self {
            snapshot_depth: DEFAULT_SNAPSHOT_DEPTH,
        }
    }
}

impl MexcSpot {
    /// Create a new `MexcSpot` with the specified snapshot depth.
    ///
    /// ### Arguments
    /// * `depth` - Snapshot depth limit. Valid values: 5, 10, 20, 50, 100, 500, 1000.
    ///
    /// ### Example
    /// ```ignore
    /// let mexc = MexcSpot::with_snapshot_depth(1000);
    /// ```
    pub fn with_snapshot_depth(depth: u32) -> Self {
        Self {
            snapshot_depth: depth,
        }
    }
}

impl Connector for MexcSpot {
    const ID: ExchangeId = ExchangeId::Mexc;

    type Channel = MexcChannel;
    type Market = MexcMarket;
    type Subscriber = WebSocketSubscriber;
    type SubValidator = WebSocketSubValidator;
    type SubResponse = MexcSubResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(MEXC_WS_URL).map_err(SocketError::UrlParse)
    }

    fn ping_interval() -> Option<PingInterval> {
        // MEXC requires ping within 60 seconds, we send every 30
        Some(PingInterval {
            interval: tokio::time::interval(Duration::from_secs(30)),
            ping: || WsMessage::text(r#"{"method":"PING"}"#),
        })
    }

    fn requests(exchange_subs: Vec<ExchangeSub<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        // Build subscription channels
        let channels: Vec<String> = exchange_subs
            .into_iter()
            .map(|sub| sub.channel.subscription_channel(sub.market.as_ref()))
            .collect();

        // MEXC uses a single subscription message with all channels
        vec![WsMessage::text(
            serde_json::json!({
                "method": "SUBSCRIPTION",
                "params": channels
            })
            .to_string(),
        )]
    }

    fn expected_responses<InstrumentKey>(map: &Map<InstrumentKey>) -> usize {
        // MEXC sends one response per subscription
        map.0.len()
    }

    fn subscription_timeout() -> Duration {
        Duration::from_secs(10)
    }
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL1> for MexcSpot
where
    Instrument: InstrumentData,
{
    type SnapFetcher = NoInitialSnapshots;
    type Stream = MexcWsStream<MexcOrderBooksL1Transformer<Instrument::Key>>;
}

impl<Instrument> StreamSelector<Instrument, OrderBooksL2> for MexcSpot
where
    Instrument: InstrumentData,
{
    // Snapshot fetching is handled internally by the transformer using deferred approach
    type SnapFetcher = NoInitialSnapshots;
    type Stream = MexcWsStream<MexcOrderBooksL2Transformer<Instrument::Key>>;
}

impl serde::Serialize for MexcSpot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(Self::ID.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for MexcSpot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <String as serde::Deserialize>::deserialize(deserializer)?;
        if input.as_str() == Self::ID.as_str() {
            Ok(Self::default())
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(input.as_str()),
                &Self::ID.as_str(),
            ))
        }
    }
}

impl std::fmt::Display for MexcSpot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MexcSpot")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mexc_spot_default() {
        let mexc = MexcSpot::default();
        assert_eq!(mexc.snapshot_depth, DEFAULT_SNAPSHOT_DEPTH);
        assert_eq!(mexc.snapshot_depth, 500);
    }

    #[test]
    fn test_mexc_spot_with_custom_depths() {
        // Test various valid depth values
        for depth in [5, 10, 20, 50, 100, 500, 1000] {
            let mexc = MexcSpot::with_snapshot_depth(depth);
            assert_eq!(mexc.snapshot_depth, depth);
        }
    }

    #[test]
    fn test_mexc_spot_serialize_deserialize() {
        let mexc = MexcSpot::with_snapshot_depth(1000);
        let serialized = serde_json::to_string(&mexc).unwrap();
        assert_eq!(serialized, "\"mexc\"");

        // Deserialization creates default instance
        let deserialized: MexcSpot = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.snapshot_depth, DEFAULT_SNAPSHOT_DEPTH);
    }
}


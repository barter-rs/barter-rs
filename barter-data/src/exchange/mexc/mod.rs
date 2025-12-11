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
    subscription::{
        Map,
        book::{OrderBooksL1, OrderBooksL2},
    },
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{WebSocketProtobufParser, WsMessage},
};
use std::{collections::HashMap, sync::RwLock, time::Duration};
use url::Url;

/// Global registry for MEXC L2 snapshot depths per symbol.
///
/// This is populated during subscription creation and read during transformer initialization.
/// Using RwLock for thread-safe access.
static MEXC_L2_SNAPSHOT_DEPTHS: RwLock<Option<HashMap<String, u32>>> = RwLock::new(None);

/// Register a snapshot depth for a symbol.
pub(crate) fn register_snapshot_depth(symbol: &str, depth: u32) {
    let mut guard = MEXC_L2_SNAPSHOT_DEPTHS.write().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(symbol.to_uppercase(), depth);
}

/// Get the registered snapshot depth for a symbol, or return the default.
pub(crate) fn get_snapshot_depth(symbol: &str) -> u32 {
    MEXC_L2_SNAPSHOT_DEPTHS
        .read()
        .unwrap()
        .as_ref()
        .and_then(|map| map.get(&symbol.to_uppercase()).copied())
        .unwrap_or(MexcSnapshotDepth::default() as u32)
}

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

/// Type alias for MEXC WebSocket stream using Protobuf parser.
pub type MexcWsStream<Transformer> = ExchangeWsStream<WebSocketProtobufParser, Transformer>;

/// Snapshot depth for MEXC L2 orderbook REST API requests.
///
/// These are the valid depth values supported by MEXC's `/api/v3/depth` endpoint.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum MexcSnapshotDepth {
    Depth5 = 5,
    Depth10 = 10,
    Depth20 = 20,
    Depth50 = 50,
    Depth100 = 100,
    #[default]
    Depth500 = 500,
    Depth1000 = 1000,
}

/// MEXC Spot exchange connector.
///
/// ### Configuration
/// The snapshot depth for L2 orderbooks can be configured:
/// ```ignore
/// use barter_data::exchange::mexc::{MexcSpot, MexcSnapshotDepth};
///
/// // Use default depth (500)
/// let mexc = MexcSpot::default();
///
/// // Use custom depth
/// let mexc = MexcSpot::with_snapshot_depth(MexcSnapshotDepth::Depth1000);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct MexcSpot {
    /// Depth limit for L2 orderbook snapshots.
    pub snapshot_depth: MexcSnapshotDepth,
}

impl MexcSpot {
    /// Create a new `MexcSpot` with the specified snapshot depth.
    ///
    /// ### Example
    /// ```ignore
    /// use barter_data::exchange::mexc::{MexcSpot, MexcSnapshotDepth};
    ///
    /// let mexc = MexcSpot::with_snapshot_depth(MexcSnapshotDepth::Depth1000);
    /// ```
    pub const fn with_snapshot_depth(depth: MexcSnapshotDepth) -> Self {
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
        // Build subscription channels and register snapshot depths for L2
        let channels: Vec<String> = exchange_subs
            .into_iter()
            .map(|sub| {
                // Register snapshot depth for L2 orderbook channels
                if let MexcChannel::AggreDepth { snapshot_depth, .. } = &sub.channel {
                    register_snapshot_depth(sub.market.as_ref(), *snapshot_depth as u32);
                }
                sub.channel.subscription_channel(sub.market.as_ref())
            })
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
        assert_eq!(mexc.snapshot_depth, MexcSnapshotDepth::Depth500);
        assert_eq!(mexc.snapshot_depth as u32, 500);
    }

    #[test]
    fn test_mexc_spot_with_custom_depth() {
        let mexc = MexcSpot::with_snapshot_depth(MexcSnapshotDepth::Depth1000);
        assert_eq!(mexc.snapshot_depth, MexcSnapshotDepth::Depth1000);
        assert_eq!(mexc.snapshot_depth as u32, 1000);
    }

    #[test]
    fn test_mexc_snapshot_depth_values() {
        // Verify all enum variants have correct numeric values
        assert_eq!(MexcSnapshotDepth::Depth5 as u32, 5);
        assert_eq!(MexcSnapshotDepth::Depth10 as u32, 10);
        assert_eq!(MexcSnapshotDepth::Depth20 as u32, 20);
        assert_eq!(MexcSnapshotDepth::Depth50 as u32, 50);
        assert_eq!(MexcSnapshotDepth::Depth100 as u32, 100);
        assert_eq!(MexcSnapshotDepth::Depth500 as u32, 500);
        assert_eq!(MexcSnapshotDepth::Depth1000 as u32, 1000);
    }

    #[test]
    fn test_mexc_spot_serialize_deserialize() {
        let mexc = MexcSpot::with_snapshot_depth(MexcSnapshotDepth::Depth1000);
        let serialized = serde_json::to_string(&mexc).unwrap();
        assert_eq!(serialized, "\"mexc\"");

        // Deserialization creates default instance (snapshot_depth is not persisted)
        let deserialized: MexcSpot = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.snapshot_depth, MexcSnapshotDepth::default());
    }
}

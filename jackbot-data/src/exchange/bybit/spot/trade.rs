//! Public trade stream types for Bybit Spot.
//!
//! Provides a [`StatelessTransformer`](crate::transformer::stateless::StatelessTransformer)
//! implementation for converting raw Bybit WebSocket trade messages into
//! normalised [`MarketEvent`](crate::event::MarketEvent)s.

use crate::{
    transformer::stateless::StatelessTransformer,
    subscription::trade::PublicTrades,
    ExchangeWsStream,
};
use super::BybitSpot;

pub use super::super::{message::BybitMessage, trade::BybitTrade};

/// [`ExchangeTransformer`](crate::transformer::ExchangeTransformer) used to
/// convert Bybit Spot WebSocket trade messages into [`PublicTrade`](PublicTrades)
/// events.
pub type BybitSpotTradesTransformer<InstrumentKey> =
    StatelessTransformer<BybitSpot, InstrumentKey, PublicTrades, BybitMessage>;

/// Type alias for a Bybit Spot trades WebSocket stream.
pub type BybitSpotTradesStream<InstrumentKey> =
    ExchangeWsStream<BybitSpotTradesTransformer<InstrumentKey>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::MarketEvent,
        subscription::Map,
        transformer::ExchangeTransformer,
    };
    use fnv::FnvHashMap;
    use jackbot_instrument::Side;
    use jackbot_integration::subscription::SubscriptionId;
    use tokio::sync::mpsc;

    fn example_trade_json() -> &'static str {
        r#"{
            "topic": "publicTrade.BTCUSDT",
            "type": "snapshot",
            "ts": 1672304486868,
            "data": [{
                "T": 1672304486865,
                "s": "BTCUSDT",
                "S": "Buy",
                "v": "0.001",
                "p": "16578.50",
                "L": "PlusTick",
                "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                "BT": false
            }]
        }"#
    }

    #[tokio::test]
    async fn test_transformer_success() {
        let sub_id = SubscriptionId::from("publicTrade|BTCUSDT");
        let mut map = FnvHashMap::default();
        map.insert(sub_id.clone(), "BTCUSDT".to_string());
        let map = Map(map);

        let (tx, _rx) = mpsc::unbounded_channel();
        let mut transformer =
            BybitSpotTradesTransformer::init(map, &[], tx).await.unwrap();

        let msg: BybitMessage = serde_json::from_str(example_trade_json()).unwrap();
        let events = transformer.transform(msg);

        assert_eq!(events.len(), 1);
        let MarketEvent { kind, .. } = events.into_iter().next().unwrap().unwrap();
        assert_eq!(kind.price, 16578.50);
        assert_eq!(kind.amount, 0.001);
        assert_eq!(kind.side, Side::Buy);
    }

    #[tokio::test]
    async fn test_transformer_unidentifiable() {
        let map = Map(FnvHashMap::<SubscriptionId, String>::default());
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut transformer =
            BybitSpotTradesTransformer::init(map, &[], tx).await.unwrap();

        let msg: BybitMessage = serde_json::from_str(example_trade_json()).unwrap();
        let events = transformer.transform(msg);

        assert_eq!(events.len(), 1);
        assert!(events[0].is_err());
    }
}

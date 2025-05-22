//! Trade event types for Bitget Spot.

use crate::{
    transformer::stateless::StatelessTransformer,
    subscription::trade::PublicTrades,
    ExchangeWsStream,
};
use super::BitgetSpot;

pub use super::super::trade::BitgetTrade;

/// [`ExchangeTransformer`](crate::transformer::ExchangeTransformer) used to
/// convert Bitget Spot WebSocket trade messages into [`PublicTrade`](PublicTrades)
/// events.
pub type BitgetSpotTradesTransformer<InstrumentKey> =
    StatelessTransformer<BitgetSpot, InstrumentKey, PublicTrades, BitgetTrade>;

/// Type alias for a Bitget Spot trades WebSocket stream.
pub type BitgetSpotTradesStream<InstrumentKey> =
    ExchangeWsStream<BitgetSpotTradesTransformer<InstrumentKey>>;

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
            \"instId\": \"ETHUSDT\",
            \"tradeId\": \"1000000000\",
            \"px\": \"1000.01\",
            \"sz\": \"0.5\",
            \"side\": \"buy\",
            \"ts\": \"1749354825200\"
        }"#
    }

    #[tokio::test]
    async fn test_transformer_success() {
        let sub_id = SubscriptionId::from("trade|ETHUSDT");
        let mut map = FnvHashMap::default();
        map.insert(sub_id.clone(), "ETHUSDT".to_string());
        let map = Map(map);

        let (tx, _rx) = mpsc::unbounded_channel();
        let mut transformer =
            BitgetSpotTradesTransformer::init(map, &[], tx).await.unwrap();

        let trade: BitgetTrade = serde_json::from_str(example_trade_json()).unwrap();
        let events = transformer.transform(trade);

        assert_eq!(events.len(), 1);
        let MarketEvent { kind, .. } = events.into_iter().next().unwrap().unwrap();
        assert_eq!(kind.price, 1000.01);
        assert_eq!(kind.amount, 0.5);
        assert_eq!(kind.side, Side::Buy);
    }

    #[tokio::test]
    async fn test_transformer_unidentifiable() {
        let map = Map(FnvHashMap::<SubscriptionId, String>::default());
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut transformer =
            BitgetSpotTradesTransformer::init(map, &[], tx).await.unwrap();

        let trade: BitgetTrade = serde_json::from_str(example_trade_json()).unwrap();
        let events = transformer.transform(trade);

        assert_eq!(events.len(), 1);
        assert!(events[0].is_err());
    }
}

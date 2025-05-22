//! Trade event types for Bitget Futures.

use crate::{
    transformer::stateless::StatelessTransformer,
    subscription::trade::PublicTrades,
    ExchangeWsStream,
};
use super::BitgetFutures;

pub use super::super::trade::BitgetTrade;

/// [`ExchangeTransformer`](crate::transformer::ExchangeTransformer) used to
/// convert Bitget Futures WebSocket trade messages into [`PublicTrade`](PublicTrades)
/// events.
pub type BitgetFuturesTradesTransformer<InstrumentKey> =
    StatelessTransformer<BitgetFutures, InstrumentKey, PublicTrades, BitgetTrade>;

/// Type alias for a Bitget Futures trades WebSocket stream.
pub type BitgetFuturesTradesStream<InstrumentKey> =
    ExchangeWsStream<BitgetFuturesTradesTransformer<InstrumentKey>>;

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
            \"instId\": \"ETHUSDT_UMCBL\",
            \"tradeId\": \"1000000000\",
            \"px\": \"1000.01\",
            \"sz\": \"0.5\",
            \"side\": \"sell\",
            \"ts\": \"1749354825200\"
        }"#
    }

    #[tokio::test]
    async fn test_transformer_success() {
        let sub_id = SubscriptionId::from("trade|ETHUSDT_UMCBL");
        let mut map = FnvHashMap::default();
        map.insert(sub_id.clone(), "ETHUSDT_UMCBL".to_string());
        let map = Map(map);

        let (tx, _rx) = mpsc::unbounded_channel();
        let mut transformer =
            BitgetFuturesTradesTransformer::init(map, &[], tx).await.unwrap();

        let trade: BitgetTrade = serde_json::from_str(example_trade_json()).unwrap();
        let events = transformer.transform(trade);

        assert_eq!(events.len(), 1);
        let MarketEvent { kind, .. } = events.into_iter().next().unwrap().unwrap();
        assert_eq!(kind.price, 1000.01);
        assert_eq!(kind.amount, 0.5);
        assert_eq!(kind.side, Side::Sell);
    }

    #[tokio::test]
    async fn test_transformer_unidentifiable() {
        let map = Map(FnvHashMap::<SubscriptionId, String>::default());
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut transformer =
            BitgetFuturesTradesTransformer::init(map, &[], tx).await.unwrap();

        let trade: BitgetTrade = serde_json::from_str(example_trade_json()).unwrap();
        let events = transformer.transform(trade);

        assert_eq!(events.len(), 1);
        assert!(events[0].is_err());
    }
}

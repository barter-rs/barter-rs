//! Public trade stream types for Binance Futures.
//!
//! This module exposes a [`StatelessTransformer`](crate::transformer::stateless::StatelessTransformer)
//! based implementation for transforming raw Binance Futures trade messages into
//! normalised [`MarketEvent`](crate::event::MarketEvent)s.

use crate::{
    transformer::stateless::StatelessTransformer,
    subscription::trade::PublicTrades,
    ExchangeWsStream,
};
use super::BinanceFuturesUsd;

pub use super::super::trade::BinanceTrade;

/// [`ExchangeTransformer`](crate::transformer::ExchangeTransformer) used to
/// convert Binance Futures WebSocket trade messages into [`PublicTrade`](PublicTrades)
/// events.
pub type BinanceFuturesTradesTransformer<InstrumentKey> =
    StatelessTransformer<BinanceFuturesUsd, InstrumentKey, PublicTrades, BinanceTrade>;

/// Type alias for a Binance Futures trades WebSocket stream.
pub type BinanceFuturesTradesStream<InstrumentKey> =
    ExchangeWsStream<BinanceFuturesTradesTransformer<InstrumentKey>>;

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
            "e":"trade","E":1649324825173,"s":"ETHUSDT","t":1000000000,
            "p":"10000.19","q":"0.239000","b":10108767791,"a":10108764858,
            "T":1749354825200,"m":true,"M":true
        }"#
    }

    #[tokio::test]
    async fn test_transformer_success() {
        let sub_id = SubscriptionId::from("@trade|ETHUSDT");
        let mut map = FnvHashMap::default();
        map.insert(sub_id.clone(), "ETHUSDT".to_string());
        let map = Map(map);

        let (tx, _rx) = mpsc::unbounded_channel();
        let mut transformer =
            BinanceFuturesTradesTransformer::init(map, &[], tx).await.unwrap();

        let trade: BinanceTrade = serde_json::from_str(example_trade_json()).unwrap();
        let events = transformer.transform(trade);

        assert_eq!(events.len(), 1);
        let MarketEvent { kind, .. } = events.into_iter().next().unwrap().unwrap();
        assert_eq!(kind.side, Side::Sell);
    }

    #[tokio::test]
    async fn test_transformer_unidentifiable() {
        let map = Map(FnvHashMap::<SubscriptionId, String>::default());
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut transformer =
            BinanceFuturesTradesTransformer::init(map, &[], tx).await.unwrap();

        let trade: BinanceTrade = serde_json::from_str(example_trade_json()).unwrap();
        let events = transformer.transform(trade);

        assert_eq!(events.len(), 1);
        assert!(events[0].is_err());
    }
}

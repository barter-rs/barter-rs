use barter_instrument::Side;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KrakenFuturesTrade {
    pub uid: String,
    pub side: Side,
    #[serde(alias = "type")]
    pub trade_type: KrakenFuturesTradeType,
    pub price: Decimal,
    pub qty: Decimal,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub time: DateTime<Utc>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum KrakenFuturesTradeType {
    Fill,
    Liquidation,
    Assignment,
    Termination,
    Block,
    #[serde(other)]
    Unknown,
}

impl KrakenFuturesTradeType {
    /// Returns true if this trade type represents a liquidation event
    pub fn is_liquidation(&self) -> bool {
        matches!(self, KrakenFuturesTradeType::Liquidation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    mod de {
        use super::*;

        #[test]
        fn test_kraken_futures_trade_deserialize_fill() {
            let json = r#"{
                "uid": "trade123",
                "side": "buy",
                "type": "fill",
                "price": "50000.00",
                "qty": "1.5",
                "time": 1234567890123
            }"#;

            let trade: KrakenFuturesTrade = serde_json::from_str(json).unwrap();
            assert_eq!(trade.uid, "trade123");
            assert_eq!(trade.side, Side::Buy);
            assert_eq!(trade.trade_type, KrakenFuturesTradeType::Fill);
            assert_eq!(trade.price, dec!(50000.00));
            assert_eq!(trade.qty, dec!(1.5));
        }

        #[test]
        fn test_kraken_futures_trade_deserialize_liquidation() {
            let json = r#"{
                "uid": "liq456",
                "side": "sell",
                "type": "liquidation",
                "price": "49500.00",
                "qty": "5.0",
                "time": 1234567890124
            }"#;

            let trade: KrakenFuturesTrade = serde_json::from_str(json).unwrap();
            assert_eq!(trade.uid, "liq456");
            assert_eq!(trade.side, Side::Sell);
            assert_eq!(trade.trade_type, KrakenFuturesTradeType::Liquidation);
            assert_eq!(trade.price, dec!(49500.00));
            assert_eq!(trade.qty, dec!(5.0));
        }

        #[test]
        fn test_kraken_futures_trade_type_unknown() {
            let json = r#"{
                "uid": "unknown123",
                "side": "buy",
                "type": "some_new_type",
                "price": "50000.00",
                "qty": "1.0",
                "time": 1234567890125
            }"#;

            let trade: KrakenFuturesTrade = serde_json::from_str(json).unwrap();
            assert_eq!(trade.trade_type, KrakenFuturesTradeType::Unknown);
        }
    }

    #[test]
    fn test_trade_type_is_liquidation() {
        assert!(KrakenFuturesTradeType::Liquidation.is_liquidation());
        assert!(!KrakenFuturesTradeType::Fill.is_liquidation());
        assert!(!KrakenFuturesTradeType::Assignment.is_liquidation());
        assert!(!KrakenFuturesTradeType::Termination.is_liquidation());
        assert!(!KrakenFuturesTradeType::Block.is_liquidation());
        assert!(!KrakenFuturesTradeType::Unknown.is_liquidation());
    }

    #[test]
    fn test_kraken_futures_trade_type_variants() {
        // Test all known trade type variants deserialize correctly
        let types = vec![
            (r#""fill""#, KrakenFuturesTradeType::Fill),
            (r#""liquidation""#, KrakenFuturesTradeType::Liquidation),
            (r#""assignment""#, KrakenFuturesTradeType::Assignment),
            (r#""termination""#, KrakenFuturesTradeType::Termination),
            (r#""block""#, KrakenFuturesTradeType::Block),
        ];

        for (json, expected) in types {
            let trade_type: KrakenFuturesTradeType = serde_json::from_str(json).unwrap();
            assert_eq!(trade_type, expected);
        }
    }
}
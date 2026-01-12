use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KrakenFuturesOrderBookL1 {
    // Placeholder - exact fields need to be verified against API
    // Assuming typical ticker fields
    pub bid: Decimal,
    pub ask: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_kraken_futures_order_book_l1_deserialize() {
        let json = r#"{
            "bid": "50000.50",
            "ask": "50001.00",
            "bid_size": "10.5",
            "ask_size": "12.3"
        }"#;

        let l1: KrakenFuturesOrderBookL1 = serde_json::from_str(json).unwrap();
        assert_eq!(l1.bid, dec!(50000.50));
        assert_eq!(l1.ask, dec!(50001.00));
        assert_eq!(l1.bid_size, dec!(10.5));
        assert_eq!(l1.ask_size, dec!(12.3));
    }

    #[test]
    fn test_kraken_futures_order_book_l1_spread() {
        let l1 = KrakenFuturesOrderBookL1 {
            bid: dec!(50000.00),
            ask: dec!(50001.50),
            bid_size: dec!(10.0),
            ask_size: dec!(15.0),
        };
        
        let spread = l1.ask - l1.bid;
        assert_eq!(spread, dec!(1.50));
    }

    #[test]
    fn test_kraken_futures_order_book_l1_high_precision() {
        let json = r#"{
            "bid": "50000.123456",
            "ask": "50000.234567",
            "bid_size": "100.987654",
            "ask_size": "200.123456"
        }"#;

        let l1: KrakenFuturesOrderBookL1 = serde_json::from_str(json).unwrap();
        assert_eq!(l1.bid, dec!(50000.123456));
        assert_eq!(l1.ask, dec!(50000.234567));
    }
}

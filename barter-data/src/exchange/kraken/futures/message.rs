use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KrakenFuturesMessage<T> {
    pub feed: String,
    pub product_id: String,
    #[serde(flatten)]
    pub payload: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestPayload {
        value: Decimal,
        name: String,
    }

    #[test]
    fn test_kraken_futures_message_deserialize() {
        let json = r#"{
            "feed": "trade",
            "product_id": "PI_XBTUSD",
            "value": "100.5",
            "name": "test"
        }"#;

        let msg: KrakenFuturesMessage<TestPayload> = serde_json::from_str(json).unwrap();
        assert_eq!(msg.feed, "trade");
        assert_eq!(msg.product_id, "PI_XBTUSD");
        assert_eq!(msg.payload.value, dec!(100.5));
        assert_eq!(msg.payload.name, "test");
    }

    #[test]
    fn test_kraken_futures_message_different_feeds() {
        let feeds = vec!["trade", "ticker", "book", "book_snapshot"];
        
        for feed in feeds {
            let json = format!(r#"{{
                "feed": "{}",
                "product_id": "PI_ETHUSD",
                "value": "1.0",
                "name": "test"
            }}"#, feed);

            let msg: KrakenFuturesMessage<TestPayload> = serde_json::from_str(&json).unwrap();
            assert_eq!(msg.feed, feed);
        }
    }

    #[test]
    fn test_kraken_futures_message_product_ids() {
        let products = vec!["PI_XBTUSD", "PI_ETHUSD", "FI_XBTUSD_240315", "PF_SOLUSD"];
        
        for product in products {
            let json = format!(r#"{{
                "feed": "trade",
                "product_id": "{}",
                "value": "1.0",
                "name": "test"
            }}"#, product);

            let msg: KrakenFuturesMessage<TestPayload> = serde_json::from_str(&json).unwrap();
            assert_eq!(msg.product_id, product);
        }
    }
}
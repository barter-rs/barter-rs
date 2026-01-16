use super::KrakenFuturesLevel;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KrakenFuturesBook {
    pub seq: u64,
    #[serde(default)]
    pub bids: Vec<KrakenFuturesLevel>,
    #[serde(default)]
    pub asks: Vec<KrakenFuturesLevel>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

pub type KrakenFuturesBookSnapshot = KrakenFuturesBook;
pub type KrakenFuturesBookUpdate = KrakenFuturesBook;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_kraken_futures_book_snapshot_deserialize() {
        let json = r#"{
            "seq": 12345,
            "bids": [[50000.50, 10.5], [49999.00, 20.3]],
            "asks": [[50001.00, 15.2], [50002.00, 25.1]],
            "timestamp": 1234567890123
        }"#;

        let book: KrakenFuturesBook = serde_json::from_str(json).unwrap();
        assert_eq!(book.seq, 12345);
        assert_eq!(book.bids.len(), 2);
        assert_eq!(book.asks.len(), 2);
        assert_eq!(book.bids[0].price, dec!(50000.50));
        assert_eq!(book.bids[0].qty, dec!(10.5));
        assert_eq!(book.asks[0].price, dec!(50001.00));
        assert_eq!(book.asks[0].qty, dec!(15.2));
    }

    #[test]
    fn test_kraken_futures_book_update_deserialize() {
        let json = r#"{
            "seq": 12346,
            "bids": [[50000.50, 5.0]],
            "asks": [],
            "timestamp": 1234567890124
        }"#;

        let book: KrakenFuturesBook = serde_json::from_str(json).unwrap();
        assert_eq!(book.seq, 12346);
        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.asks.len(), 0);
    }

    #[test]
    fn test_kraken_futures_book_empty_sides() {
        let json = r#"{
            "seq": 12347,
            "bids": [],
            "asks": [],
            "timestamp": 1234567890125
        }"#;

        let book: KrakenFuturesBook = serde_json::from_str(json).unwrap();
        assert_eq!(book.seq, 12347);
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_kraken_futures_book_zero_quantity_removal() {
        // Zero quantity indicates removal of a price level
        let json = r#"{
            "seq": 12348,
            "bids": [[50000.50, 0]],
            "asks": [[50001.00, 0]],
            "timestamp": 1234567890126
        }"#;

        let book: KrakenFuturesBook = serde_json::from_str(json).unwrap();
        assert_eq!(book.bids[0].qty, dec!(0));
        assert_eq!(book.asks[0].qty, dec!(0));
    }

    #[test]
    fn test_kraken_futures_book_sequence_ordering() {
        // Verify sequence numbers are correctly parsed for delta ordering
        let json1 = r#"{"seq": 1, "bids": [], "asks": [], "timestamp": 1234567890001}"#;
        let json2 = r#"{"seq": 2, "bids": [], "asks": [], "timestamp": 1234567890002}"#;

        let book1: KrakenFuturesBook = serde_json::from_str(json1).unwrap();
        let book2: KrakenFuturesBook = serde_json::from_str(json2).unwrap();
        
        assert!(book2.seq > book1.seq);
    }

    #[test]
    fn test_kraken_futures_book_default_sides() {
        // Test default values when sides are missing
        let json = r#"{
            "seq": 12349,
            "timestamp": 1234567890127
        }"#;

        let book: KrakenFuturesBook = serde_json::from_str(json).unwrap();
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn test_kraken_futures_book_type_aliases() {
        // Verify type aliases work correctly
        let json = r#"{
            "seq": 12350,
            "bids": [[50000.00, 1.0]],
            "asks": [[50001.00, 1.0]],
            "timestamp": 1234567890128
        }"#;

        let snapshot: KrakenFuturesBookSnapshot = serde_json::from_str(json).unwrap();
        let update: KrakenFuturesBookUpdate = serde_json::from_str(json).unwrap();
        
        assert_eq!(snapshot.seq, update.seq);
    }
}

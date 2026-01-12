use rust_decimal::Decimal;

pub mod l1;
pub mod l2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KrakenFuturesLevel {
    pub price: Decimal,
    pub qty: Decimal,
}

impl<'de> serde::Deserialize<'de> for KrakenFuturesLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (price, qty) = <(Decimal, Decimal)>::deserialize(deserializer)?;
        Ok(KrakenFuturesLevel { price, qty })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_kraken_futures_level_deserialize() {
        let json = r#"[50000.50, 1.25]"#;
        let level: KrakenFuturesLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level.price, dec!(50000.50));
        assert_eq!(level.qty, dec!(1.25));
    }

    #[test]
    fn test_kraken_futures_level_zero_qty() {
        // Zero qty means remove order at that price
        let json = r#"[50000.50, 0]"#;
        let level: KrakenFuturesLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level.price, dec!(50000.50));
        assert_eq!(level.qty, dec!(0));
    }

    #[test]
    fn test_kraken_futures_level_high_precision() {
        let json = r#"[50000.123456789, 1.987654321]"#;
        let level: KrakenFuturesLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level.price, dec!(50000.123456789));
        assert_eq!(level.qty, dec!(1.987654321));
    }

    #[test]
    fn test_kraken_futures_level_equality() {
        let level1 = KrakenFuturesLevel { price: dec!(50000), qty: dec!(1.5) };
        let level2 = KrakenFuturesLevel { price: dec!(50000), qty: dec!(1.5) };
        let level3 = KrakenFuturesLevel { price: dec!(50001), qty: dec!(1.5) };
        
        assert_eq!(level1, level2);
        assert_ne!(level1, level3);
    }

    #[test]
    fn test_kraken_futures_level_clone() {
        let level1 = KrakenFuturesLevel { price: dec!(50000), qty: dec!(1.5) };
        let level2 = level1;
        assert_eq!(level1, level2);
    }
}

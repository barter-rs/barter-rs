use rust_decimal::Decimal;
use serde::Deserialize;

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

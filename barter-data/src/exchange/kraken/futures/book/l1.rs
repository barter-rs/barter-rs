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

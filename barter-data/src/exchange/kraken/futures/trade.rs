use barter_integration::model::Side;
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

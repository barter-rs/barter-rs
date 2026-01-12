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
    pub time: DateTime<Utc>,
}

pub type KrakenFuturesBookSnapshot = KrakenFuturesBook;
pub type KrakenFuturesBookUpdate = KrakenFuturesBook;

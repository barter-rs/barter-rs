use crate::{
    exchange::{bybit::channel::BybitChannel, subscription::ExchangeSub},
    subscription::book::{Level, OrderBook, OrderBookSide},
    Identifier,
};
use barter_integration::model::{Side, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookL2 {
    pub topic: String,
    #[serde(rename = "type")]
    pub update_type: BybitOrderBookL2Type,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub ts: DateTime<Utc>,
    pub data: BybitOrderBookL2Data,
    pub cts: u64,
}

impl Identifier<Option<SubscriptionId>> for BybitOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.data.subscription_id.clone())
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub enum BybitOrderBookL2Type {
    #[serde(alias = "snapshot")]
    Snapshot,
    #[serde(alias = "delta")]
    Delta,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookL2Data {
    #[serde(alias = "s", deserialize_with = "de_ob_l2_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(deserialize_with = "de_ob_l2_levels")]
    pub b: Vec<BybitLevel>,
    #[serde(deserialize_with = "de_ob_l2_levels")]
    pub a: Vec<BybitLevel>,
    pub u: u64,
    pub seq: u64,
}

/// Deserialize a [`BybitOrderBookL2`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`].
///
/// eg/ "orderbook.50.BTCUSDT"
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BybitChannel::ORDER_BOOK_L2, market)).id())
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLevel(pub f64, pub f64);

impl From<BybitLevel> for Level {
    fn from(bybit_level: BybitLevel) -> Self {
        Level {
            price: bybit_level.0,
            amount: bybit_level.1,
        }
    }
}

pub fn de_ob_l2_levels<'de, D>(deserializer: D) -> Result<Vec<BybitLevel>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let levels: Vec<[String; 2]> = Vec::<[String; 2]>::deserialize(deserializer)?;

    levels
        .into_iter()
        .map(|[price, amount]| {
            Ok(BybitLevel(
                price.parse().map_err(serde::de::Error::custom)?,
                amount.parse().map_err(serde::de::Error::custom)?,
            ))
        })
        .collect()
}

impl From<BybitOrderBookL2> for OrderBook {
    fn from(snapshot: BybitOrderBookL2) -> Self {
        Self {
            last_update_time: snapshot.ts,
            bids: OrderBookSide::new(Side::Buy, snapshot.data.b),
            asks: OrderBookSide::new(Side::Sell, snapshot.data.a),
        }
    }
}

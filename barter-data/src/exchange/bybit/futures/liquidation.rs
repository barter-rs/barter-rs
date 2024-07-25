use super::super::BybitChannel;
use crate::{
    event::{MarketEvent, MarketIter},
    exchange::ExchangeId,
    subscription::liquidation::Liquidation,
    Identifier,
};
use barter_integration::model::{Exchange, Side, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`BybitFuturesUsd`](super::BybitFuturesUsd) Liquidation message.
///
/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/liquidation>
/// ```json
/// {
///     "topic": "liquidation.BTCUSDT",
///     "type": "snapshot",
///     "ts": 1703485237953,
///     "data": {
///         "updatedTime": 1703485237953,
///         "symbol": "BTCUSDT",
///         "side": "Sell",
///         "size": "0.003",
///         "price": "43511.70"
///     }
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLiquidation {
    pub topic: String,
    #[serde(alias = "type")]
    pub r#type: String,
    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    #[serde(alias = "data")]
    pub data: BybitLiquidationData,
}

/// [`BybitFuturesUsd`](super::BybitFuturesUsd) Liquidation data.
///
/// ### Raw Payload Examples
/// ```json
/// {
///    "updatedTime": 1703485237953,
///    "symbol": "BTCUSDT",
///    "side": "Sell",
///    "size": "0.003",
///    "price": "43511.70"
/// }
/// ```
///
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLiquidationData {
    #[serde(alias = "symbol", deserialize_with = "de_liquidation_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "side")]
    pub side: Side,
    #[serde(alias = "price", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(alias = "size", deserialize_with = "barter_integration::de::de_str")]
    pub size: f64,
    #[serde(
        alias = "updatedTime",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub updated_time: DateTime<Utc>,
}

impl Identifier<Option<SubscriptionId>> for BybitLiquidation {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.data.subscription_id.clone())
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, BybitLiquidation)>
    for MarketIter<InstrumentId, Liquidation>
{
    fn from(
        (exchange_id, instrument, liquidation): (ExchangeId, InstrumentId, BybitLiquidation),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_time: liquidation.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: Liquidation {
                side: liquidation.data.side,
                price: liquidation.data.price,
                quantity: liquidation.data.size,
                time: liquidation.data.updated_time,
            },
        })])
    }
}

/// Deserialize a [`BybitLiquidationData`] "s" (eg/ "BTCUSDT") as the associated
/// [`SubscriptionId`].
///
/// eg/ "liquidation.BTCUSDT"
pub fn de_liquidation_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(|market: String| {
        SubscriptionId::from(format!("{}.{}", BybitChannel::LIQUIDATIONS.0, market))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use barter_integration::de::datetime_utc_from_epoch_duration;
        use std::time::Duration;

        #[test]
        fn test_bybit_liquidation() {
            let input = r#"
            {
                "topic": "liquidation.BTCUSDT",
                "type": "snapshot",
                "ts": 1703485237953,
                "data": {
                    "updatedTime": 1703485237953,
                    "symbol": "BTCUSDT",
                    "side": "Sell",
                    "size": "0.003",
                    "price": "43511.70"
                }
            }
            "#;

            let time = datetime_utc_from_epoch_duration(Duration::from_millis(1703485237953));

            assert_eq!(
                serde_json::from_str::<BybitLiquidation>(input).unwrap(),
                BybitLiquidation {
                    topic: "liquidation.BTCUSDT".to_string(),
                    r#type: "snapshot".to_string(),
                    time,
                    data: BybitLiquidationData {
                        subscription_id: SubscriptionId::from("liquidation.BTCUSDT"),
                        side: Side::Sell,
                        price: 43511.70,
                        size: 0.003,
                        updated_time: time,
                    },
                }
            );
        }
    }
}

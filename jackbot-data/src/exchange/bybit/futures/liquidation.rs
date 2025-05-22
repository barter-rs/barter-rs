use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
    subscription::liquidation::Liquidation,
};
use jackbot_instrument::{Side, exchange::ExchangeId};
use jackbot_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`BybitFuturesUsd`](super::BybitFuturesUsd) Liquidation order message.
///
/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/liquidation>
/// ```json
/// {
///     "id": "c3ab5ad1-3612-4eba-beec-b047cb35ea63",
///     "topic": "liquidation.BTCUSDT",
///     "ts": 1708996171222,
///     "data": {
///         "price": "26456.00",
///         "side": "Sell",
///         "size": "0.156",
///         "symbol": "BTCUSDT",
///         "updatedTime": 1708996171222
///     },
///     "type": "snapshot"
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLiquidation {
    pub topic: String,
    pub data: BybitLiquidationData,
}

/// [`BybitFuturesUsd`](super::BybitFuturesUsd) Liquidation order data.
///
/// ### Raw Payload Examples
/// ```json
/// {
///     "price": "26456.00",
///     "side": "Sell",
///     "size": "0.156",
///     "symbol": "BTCUSDT",
///     "updatedTime": 1708996171222
/// }
/// ```
///
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/liquidation>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLiquidationData {
    #[serde(deserialize_with = "jackbot_integration::de::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "de_side")]
    pub side: Side,
    #[serde(deserialize_with = "jackbot_integration::de::de_str")]
    pub size: f64,
    pub symbol: String,
    #[serde(deserialize_with = "jackbot_integration::de::de_u64_epoch_ms_as_datetime_utc")]
    pub updatedTime: DateTime<Utc>,
}

impl Identifier<Option<SubscriptionId>> for BybitLiquidation {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(self.topic.clone()))
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BybitLiquidation)>
    for MarketIter<InstrumentKey, Liquidation>
{
    fn from(
        (exchange_id, instrument, liquidation): (ExchangeId, InstrumentKey, BybitLiquidation),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: liquidation.data.updatedTime,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: Liquidation {
                side: liquidation.data.side,
                price: liquidation.data.price,
                quantity: liquidation.data.size,
                time: liquidation.data.updatedTime,
            },
        })])
    }
}

/// Deserialize a [`BybitLiquidationData`] "side" (eg/ "Buy" or "Sell") as the associated
/// [`Side`].
fn de_side<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let side = String::deserialize(deserializer)?;
    match side.as_str() {
        "Buy" => Ok(Side::Buy),
        "Sell" => Ok(Side::Sell),
        _ => Err(serde::de::Error::custom(format!(
            "Failed to deserialize Bybit liquidation side: {}",
            side
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use jackbot_integration::de::datetime_utc_from_epoch_duration;
        use std::time::Duration;

        #[test]
        fn test_bybit_liquidation() {
            let input = r#"
            {
                "id": "c3ab5ad1-3612-4eba-beec-b047cb35ea63",
                "topic": "liquidation.BTCUSDT",
                "ts": 1708996171222,
                "data": {
                    "price": "26456.00",
                    "side": "Sell",
                    "size": "0.156",
                    "symbol": "BTCUSDT",
                    "updatedTime": 1708996171222
                },
                "type": "snapshot"
            }
            "#;

            let liquidation = serde_json::from_str::<BybitLiquidation>(input).unwrap();

            assert_eq!(
                liquidation,
                BybitLiquidation {
                    topic: "liquidation.BTCUSDT".to_string(),
                    data: BybitLiquidationData {
                        price: 26456.00,
                        side: Side::Sell,
                        size: 0.156,
                        symbol: "BTCUSDT".to_string(),
                        updatedTime: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1708996171222,
                        )),
                    },
                }
            );
        }
    }
}

use super::super::BinanceChannel;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
    subscription::liquidation::Liquidation,
};
use barter_instrument::{Side, exchange::ExchangeId};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`BinanceFuturesUsd`](super::BinanceFuturesUsd) Liquidation order message.
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
/// ```json
/// {
///     "e": "forceOrder",
///     "E": 1665523974222,
///     "o": {
///         "s": "BTCUSDT",
///         "S": "SELL",
///         "o": "LIMIT",
///         "f": "IOC",
///         "q": "0.009",
///         "p": "18917.15",
///         "ap": "18990.00",
///         "X": "FILLED",
///         "l": "0.009",
///         "z": "0.009",
///         "T": 1665523974217
///     }
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLiquidation {
    #[serde(alias = "o")]
    pub order: BinanceLiquidationOrder,
}

/// [`BinanceFuturesUsd`](super::BinanceFuturesUsd) Liquidation order.
///
/// ### Raw Payload Examples
/// ```json
/// {
///     "s": "BTCUSDT",
///     "S": "SELL",
///     "o": "LIMIT",
///     "f": "IOC",
///     "q": "0.009",
///     "p": "18917.15",
///     "ap": "18990.00",
///     "X": "FILLED",
///     "l": "0.009",
///     "z": "0.009",
///     "T": 1665523974217
/// }
/// ```
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams>
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLiquidationOrder {
    #[serde(alias = "s", deserialize_with = "de_liquidation_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "S")]
    pub side: Side,
    #[serde(alias = "p", deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(alias = "q", deserialize_with = "barter_integration::de::de_str")]
    pub quantity: f64,
    #[serde(
        alias = "T",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
}

impl Identifier<Option<SubscriptionId>> for BinanceLiquidation {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.order.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BinanceLiquidation)>
    for MarketIter<InstrumentKey, Liquidation>
{
    fn from(
        (exchange_id, instrument, liquidation): (ExchangeId, InstrumentKey, BinanceLiquidation),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: liquidation.order.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: Liquidation {
                side: liquidation.order.side,
                price: liquidation.order.price,
                quantity: liquidation.order.quantity,
                time: liquidation.order.time,
            },
        })])
    }
}

/// Deserialize a [`BinanceLiquidationOrder`] "s" (eg/ "BTCUSDT") as the associated
/// [`SubscriptionId`].
///
/// eg/ "forceOrder|BTCUSDT"
pub fn de_liquidation_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(|market: String| {
        SubscriptionId::from(format!("{}|{}", BinanceChannel::LIQUIDATIONS.0, market))
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
        fn test_binance_liquidation() {
            let input = r#"
            {
                "e": "forceOrder",
                "E": 1665523974222,
                "o": {
                    "s": "BTCUSDT",
                    "S": "SELL",
                    "o": "LIMIT",
                    "f": "IOC",
                    "q": "0.009",
                    "p": "18917.15",
                    "ap": "18990.00",
                    "X": "FILLED",
                    "l": "0.009",
                    "z": "0.009",
                    "T": 1665523974217
                }
            }
            "#;

            assert_eq!(
                serde_json::from_str::<BinanceLiquidation>(input).unwrap(),
                BinanceLiquidation {
                    order: BinanceLiquidationOrder {
                        subscription_id: SubscriptionId::from("@forceOrder|BTCUSDT"),
                        side: Side::Sell,
                        price: 18917.15,
                        quantity: 0.009,
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1665523974217,
                        )),
                    },
                }
            );
        }
    }
}

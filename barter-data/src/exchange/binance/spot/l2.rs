use super::super::book::BinanceLevel;
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::ExchangeId;
use crate::subscription::book::{OrderBookEvent};
use crate::Identifier;
use barter_integration::model::{Exchange, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::books::OrderBook;

/// [`BinanceSpot`](super::BinanceSpot) HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#order-book>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT: &str = "https://api.binance.com/api/v3/depth";

/// [`BinanceSpot`](super::BinanceSpot) OrderBook Level2 deltas WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#partial-book-depth-streams>
/// ```json
/// {
///     "e":"depthUpdate",
///     "E":1671656397761,
///     "s":"ETHUSDT",
///     "U":22611425143,
///     "u":22611425151,
///     "b":[
///         ["1209.67000000","85.48210000"],
///         ["1209.66000000","20.68790000"]
///     ],
///     "a":[]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceSpotOrderBookL2Update {
    #[serde(
        alias = "s",
        deserialize_with = "super::super::book::l2::de_ob_l2_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "E",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time_exchange: DateTime<Utc>,
    #[serde(alias = "U")]
    pub first_update_id: u64,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceSpotOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, BinanceSpotOrderBookL2Update)>
    for MarketIter<InstrumentId, OrderBookEvent>
{
    fn from(
        (exchange_id, instrument, update): (ExchangeId, InstrumentId, BinanceSpotOrderBookL2Update),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: update.time_exchange,
            time_received: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: OrderBookEvent::Update(OrderBook::new(
                Some(update.last_update_id),
                None,
                update.bids,
                update.asks,
            )),
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_de_binance_spot_order_book_l2_update() {
        let input = r#"
            {
                "e":"depthUpdate",
                "E":1671656397761,
                "s":"ETHUSDT",
                "U":22611425143,
                "u":22611425151,
                "b":[
                    ["1209.67000000","85.48210000"],
                    ["1209.66000000","20.68790000"]
                ],
                "a":[]
            }
            "#;

        assert_eq!(
            serde_json::from_str::<BinanceSpotOrderBookL2Update>(input).unwrap(),
            BinanceSpotOrderBookL2Update {
                subscription_id: SubscriptionId::from("@depth@100ms|ETHUSDT"),
                time_exchange: DateTime::from_timestamp_millis(1671656397761).unwrap(),
                first_update_id: 22611425143,
                last_update_id: 22611425151,
                bids: vec![
                    BinanceLevel {
                        price: dec!(1209.67000000),
                        amount: dec!(85.48210000)
                    },
                    BinanceLevel {
                        price: dec!(1209.66000000),
                        amount: dec!(20.68790000)
                    },
                ],
                asks: vec![]
            }
        );
    }
}

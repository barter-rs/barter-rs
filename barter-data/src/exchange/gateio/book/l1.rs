use crate::{
    Identifier,
    books::Level,
    event::{MarketEvent, MarketIter},
    exchange::{
        gateio::{channel::GateioChannel, message::GateioMessage},
        subscription::ExchangeSub,
    },
    subscription::book::OrderBookL1,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub type GateioOrderBookL1 = GateioMessage<GateioOrderBookL1Inner>;

/// [`Gateio`](super::super::Gateio) real-time OrderBook Level1 (top of books) message.
///
/// ### Raw Payload Examples
/// #### GateioSpot OrderBookL1
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#best-bid-or-ask-price>
/// ```json
/// {
///     "t": 1606293275123,
///     "u": 48733182,
///     "s": "BTC_USDT",
///     "b": "19177.79",
///     "B": "0.0003341504",
///     "a": "19179.38",
///     "A": "0.09"
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct GateioOrderBookL1Inner {
    #[serde(alias = "s", deserialize_with = "de_ob_l1_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "t",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc",
        default = "Utc::now"
    )]
    pub time: DateTime<Utc>,
    pub u: u64,
    #[serde(alias = "b", with = "rust_decimal::serde::str")]
    pub best_bid_price: Decimal,
    #[serde(alias = "B", with = "rust_decimal::serde::str")]
    pub best_bid_amount: Decimal,
    #[serde(alias = "a", with = "rust_decimal::serde::str")]
    pub best_ask_price: Decimal,
    #[serde(alias = "A", with = "rust_decimal::serde::str")]
    pub best_ask_amount: Decimal,
}

impl Identifier<Option<SubscriptionId>> for GateioOrderBookL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.data.subscription_id.clone())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, GateioOrderBookL1)>
    for MarketIter<InstrumentKey, OrderBookL1>
{
    fn from(
        (exchange_id, instrument, book): (ExchangeId, InstrumentKey, GateioOrderBookL1),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: book.data.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: OrderBookL1 {
                last_update_time: book.data.time,
                best_bid: Some(Level {
                    amount: book.data.best_bid_amount,
                    price: book.data.best_bid_price,
                }),
                best_ask: Some(Level {
                    amount: book.data.best_ask_amount,
                    price: book.data.best_ask_price,
                }),
            },
        })])
    }
}

pub fn de_ob_l1_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((GateioChannel::ORDER_BOOK_L1, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use rust_decimal_macros::dec;

        #[test]
        fn test_gateio_order_book_l1() {
            struct TestCase {
                input: &'static str,
                expected: GateioOrderBookL1Inner,
            }

            let time = Utc::now();

            let tests = vec![
                TestCase {
                    // TC0: valid Spot GateioOrderBookL1Inner
                    input: r#"
                    {
                        "u":16710819973,
                        "s":"ETH_USDT",
                        "b":"1215.27000000",
                        "B":"32.49110000",
                        "a":"1215.28000000",
                        "A":"13.93900000"
                    }
                "#,
                    expected: GateioOrderBookL1Inner {
                        u: 16710819973,
                        subscription_id: SubscriptionId::from("spot.book_ticker|ETH_USDT"),
                        time,
                        best_bid_price: dec!(1215.27000000),
                        best_bid_amount: dec!(32.49110000),
                        best_ask_price: dec!(1215.28000000),
                        best_ask_amount: dec!(13.93900000),
                    },
                },
                TestCase {
                    // TC0: valid Spot GateioOrderBookL1Inner
                    input: r#"
                    {
                        "u":16710819974,
                        "s":"BTC_USDT",
                        "b":"16858.90",
                        "B":"13.692",
                        "a":"16859.00",
                        "A":"30.219"
                    }
                "#,
                    expected: GateioOrderBookL1Inner {
                        u: 16710819974,
                        subscription_id: SubscriptionId::from("spot.book_ticker|BTC_USDT"),
                        time,
                        best_bid_price: dec!(16858.90),
                        best_bid_amount: dec!(13.692),
                        best_ask_price: dec!(16859.00),
                        best_ask_amount: dec!(30.219),
                    },
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<GateioOrderBookL1Inner>(test.input).unwrap();
                let actual = GateioOrderBookL1Inner { time, ..actual };
                assert_eq!(actual, test.expected, "TC{} failed", index);
            }
        }
    }
}

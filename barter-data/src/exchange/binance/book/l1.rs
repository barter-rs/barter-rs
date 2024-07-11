use crate::{
    event::{MarketEvent, MarketIter},
    exchange::{binance::channel::BinanceChannel, subscription::ExchangeSub, ExchangeId},
    subscription::book::{Level, OrderBookL1},
    Identifier,
};
use barter_integration::model::{Exchange, SubscriptionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`Binance`](super::super::Binance) real-time OrderBook Level1 (top of book) message.
///
/// ### Raw Payload Examples
/// #### BinanceSpot OrderBookL1
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#individual-symbol-book-ticker-streams>
/// ```json
/// {
///     "u":22606535573,
///     "s":"ETHUSDT",
///     "b":"1215.27000000",
///     "B":"32.49110000",
///     "a":"1215.28000000",
///     "A":"13.93900000"
/// }
/// ```
///
/// #### BinanceFuturesUsd OrderBookL1
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#individual-symbol-book-ticker-streams>
/// ```json
/// {
///     "u":22606535573,
///     "s":"ETHUSDT",
///     "b":"1215.27000000",
///     "B":"32.49110000",
///     "a":"1215.28000000",
///     "A":"13.93900000"
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBookL1 {
    #[serde(alias = "s", deserialize_with = "de_ob_l1_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "T",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc",
        default = "Utc::now"
    )]
    pub time: DateTime<Utc>,
    #[serde(alias = "b", deserialize_with = "barter_integration::de::de_str")]
    pub best_bid_price: f64,
    #[serde(alias = "B", deserialize_with = "barter_integration::de::de_str")]
    pub best_bid_amount: f64,
    #[serde(alias = "a", deserialize_with = "barter_integration::de::de_str")]
    pub best_ask_price: f64,
    #[serde(alias = "A", deserialize_with = "barter_integration::de::de_str")]
    pub best_ask_amount: f64,
}

impl Identifier<Option<SubscriptionId>> for BinanceOrderBookL1 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, BinanceOrderBookL1)>
    for MarketIter<InstrumentId, OrderBookL1>
{
    fn from(
        (exchange_id, instrument, book): (ExchangeId, InstrumentId, BinanceOrderBookL1),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_time: book.time,
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: OrderBookL1 {
                last_update_time: book.time,
                best_bid: Level::new(book.best_bid_price, book.best_bid_amount),
                best_ask: Level::new(book.best_ask_price, book.best_ask_amount),
            },
        })])
    }
}

/// Deserialize a [`BinanceOrderBookL1`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`].
///
/// eg/ "@bookTicker|BTCUSDT"
pub fn de_ob_l1_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BinanceChannel::ORDER_BOOK_L1, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_binance_order_book_l1() {
            struct TestCase {
                input: &'static str,
                expected: BinanceOrderBookL1,
            }

            let time = Utc::now();

            let tests = vec![
                TestCase {
                    // TC0: valid Spot BinanceOrderBookL1
                    input: r#"
                    {
                        "u":22606535573,
                        "s":"ETHUSDT",
                        "b":"1215.27000000",
                        "B":"32.49110000",
                        "a":"1215.28000000",
                        "A":"13.93900000"
                    }
                "#,
                    expected: BinanceOrderBookL1 {
                        subscription_id: SubscriptionId::from("@bookTicker|ETHUSDT"),
                        time,
                        best_bid_price: 1215.27000000,
                        best_bid_amount: 32.49110000,
                        best_ask_price: 1215.28000000,
                        best_ask_amount: 13.93900000,
                    },
                },
                TestCase {
                    // TC1: valid FuturePerpetual BinanceOrderBookL1
                    input: r#"
                    {
                        "e":"bookTicker",
                        "u":2286618712950,
                        "s":"BTCUSDT",
                        "b":"16858.90",
                        "B":"13.692",
                        "a":"16859.00",
                        "A":"30.219",
                        "T":1671621244670,
                        "E":1671621244673
                    }"#,
                    expected: BinanceOrderBookL1 {
                        subscription_id: SubscriptionId::from("@bookTicker|BTCUSDT"),
                        time,
                        best_bid_price: 16858.90,
                        best_bid_amount: 13.692,
                        best_ask_price: 16859.00,
                        best_ask_amount: 30.219,
                    },
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BinanceOrderBookL1>(test.input).unwrap();
                let actual = BinanceOrderBookL1 { time, ..actual };
                assert_eq!(actual, test.expected, "TC{} failed", index);
            }
        }
    }
}

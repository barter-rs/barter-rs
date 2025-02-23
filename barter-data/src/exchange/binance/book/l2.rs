use super::{super::channel::BinanceChannel, BinanceLevel};
use crate::{
    Identifier, books::OrderBook, event::MarketEvent, exchange::subscription::ExchangeSub,
    subscription::book::OrderBookEvent,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Constructor)]
pub struct BinanceOrderBookL2Meta<InstrumentKey, Sequencer> {
    pub key: InstrumentKey,
    pub sequencer: Sequencer,
}

/// [`Binance`](super::super::Binance) OrderBook Level2 snapshot HTTP message.
///
/// Used as the starting [`OrderBook`] before OrderBook Level2 delta WebSocket updates are
/// applied.
///
/// ### Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#order-book>
/// #### BinanceSpot OrderBookL2Snapshot
/// ```json
/// {
///     "lastUpdateId": 1027024,
///     "bids": [
///         ["4.00000000", "431.00000000"]
///     ],
///     "asks": [
///         ["4.00000200", "12.00000000"]
///     ]
/// }
/// ```
///
/// #### BinanceFuturesUsd OrderBookL2Snapshot
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#order-book>
/// ```json
/// {
///     "lastUpdateId": 1027024,
///     "E": 1589436922972,
///     "T": 1589436922959,
///     "bids": [
///         ["4.00000000", "431.00000000"]
///     ],
///     "asks": [
///         ["4.00000200", "12.00000000"]
///     ]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceOrderBookL2Snapshot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    #[serde(default, rename = "E", with = "chrono::serde::ts_milliseconds_option")]
    pub time_exchange: Option<DateTime<Utc>>,
    #[serde(default, rename = "T", with = "chrono::serde::ts_milliseconds_option")]
    pub time_engine: Option<DateTime<Utc>>,
    pub bids: Vec<BinanceLevel>,
    pub asks: Vec<BinanceLevel>,
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BinanceOrderBookL2Snapshot)>
    for MarketEvent<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, snapshot): (ExchangeId, InstrumentKey, BinanceOrderBookL2Snapshot),
    ) -> Self {
        let time_received = Utc::now();
        Self {
            time_exchange: snapshot.time_exchange.unwrap_or(time_received),
            time_received,
            exchange,
            instrument,
            kind: OrderBookEvent::from(snapshot),
        }
    }
}

impl From<BinanceOrderBookL2Snapshot> for OrderBookEvent {
    fn from(snapshot: BinanceOrderBookL2Snapshot) -> Self {
        Self::Snapshot(OrderBook::new(
            snapshot.last_update_id,
            snapshot.time_engine,
            snapshot.bids,
            snapshot.asks,
        ))
    }
}

/// Deserialize a
/// [`BinanceSpotOrderBookL2Update`](super::super::spot::l2::BinanceSpotOrderBookL2Update) or
/// [`BinanceFuturesOrderBookL2Update`](super::super::futures::l2::BinanceFuturesOrderBookL2Update)
/// "s" field (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
///
/// eg/ "@depth@100ms|BTCUSDT"
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSub::from((BinanceChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use rust_decimal_macros::dec;

        #[test]
        fn test_binance_order_book_l2_snapshot() {
            struct TestCase {
                input: &'static str,
                expected: BinanceOrderBookL2Snapshot,
            }

            let tests = vec![
                TestCase {
                    // TC0: valid Spot BinanceOrderBookL2Snapshot
                    input: r#"
                    {
                        "lastUpdateId": 1027024,
                        "bids": [
                            [
                                "4.00000000",
                                "431.00000000"
                            ]
                        ],
                        "asks": [
                            [
                                "4.00000200",
                                "12.00000000"
                            ]
                        ]
                    }
                    "#,
                    expected: BinanceOrderBookL2Snapshot {
                        last_update_id: 1027024,
                        time_exchange: Default::default(),
                        time_engine: Default::default(),
                        bids: vec![BinanceLevel {
                            price: dec!(4.00000000),
                            amount: dec!(431.00000000),
                        }],
                        asks: vec![BinanceLevel {
                            price: dec!(4.00000200),
                            amount: dec!(12.00000000),
                        }],
                    },
                },
                TestCase {
                    // TC1: valid FuturePerpetual BinanceOrderBookL2Snapshot
                    input: r#"
                    {
                        "lastUpdateId": 1027024,
                        "E": 1589436922972,
                        "T": 1589436922959,
                        "bids": [
                            [
                                "4.00000000",
                                "431.00000000"
                            ]
                        ],
                        "asks": [
                            [
                                "4.00000200",
                                "12.00000000"
                            ]
                        ]
                    }
                    "#,
                    expected: BinanceOrderBookL2Snapshot {
                        last_update_id: 1027024,
                        time_exchange: Some(
                            DateTime::from_timestamp_millis(1589436922972).unwrap(),
                        ),
                        time_engine: Some(DateTime::from_timestamp_millis(1589436922959).unwrap()),
                        bids: vec![BinanceLevel {
                            price: dec!(4.0),
                            amount: dec!(431.0),
                        }],
                        asks: vec![BinanceLevel {
                            price: dec!(4.00000200),
                            amount: dec!(12.0),
                        }],
                    },
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    serde_json::from_str::<BinanceOrderBookL2Snapshot>(test.input).unwrap(),
                    test.expected,
                    "TC{} failed",
                    index
                );
            }
        }
    }
}

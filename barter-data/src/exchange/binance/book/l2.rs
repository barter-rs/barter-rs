use super::{super::channel::BinanceChannel, BinanceLevel};
use crate::{
    exchange::subscription::ExchangeSub,
    subscription::book::{OrderBook, OrderBookSide},
    Identifier,
};
use barter_integration::model::{Side, SubscriptionId};
use chrono::Utc;
use serde::{Deserialize, Serialize};

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
    pub bids: Vec<BinanceLevel>,
    pub asks: Vec<BinanceLevel>,
}

impl From<BinanceOrderBookL2Snapshot> for OrderBook {
    fn from(snapshot: BinanceOrderBookL2Snapshot) -> Self {
        Self {
            last_update_time: Utc::now(),
            bids: OrderBookSide::new(Side::Buy, snapshot.bids),
            asks: OrderBookSide::new(Side::Sell, snapshot.asks),
        }
    }
}

/// Deserialize a
/// [`BinanceSpotOrderBookL2Delta`](super::super::spot::l2::BinanceSpotOrderBookL2Delta) or
/// [`BinanceFuturesOrderBookL2Delta`](super::super::futures::l2::BinanceFuturesOrderBookL2Delta)
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
                        bids: vec![BinanceLevel {
                            price: 4.0,
                            amount: 431.0,
                        }],
                        asks: vec![BinanceLevel {
                            price: 4.00000200,
                            amount: 12.0,
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
                        bids: vec![BinanceLevel {
                            price: 4.0,
                            amount: 431.0,
                        }],
                        asks: vec![BinanceLevel {
                            price: 4.00000200,
                            amount: 12.0,
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

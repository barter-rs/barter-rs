use super::BybitLevel;
use crate::subscription::book::{OrderBook, OrderBookSide};
use barter_integration::model::Side;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// [`Bybit`](super::super::Bybit) OrderBook Level2 snapshot HTTP message.
///
/// Used as the starting [`OrderBook`] before OrderBook Level2 delta WebSocket updates are
/// applied.
///
/// ### Payload Examples
/// #### BybitPerpetualsUsd OrderBookL2Snapshot
/// See docs: <https://bybit-exchange.github.io/docs/v5/market/orderbook>
/// ```json
/// {
///     "retCode": 0,
///     "retMsg": "OK",
///     "result": {
///         "s": "BTCUSDT",
///         "a": [
///             [
///                 "65557.7",
///                 "16.606555"
///             ]
///         ],
///         "b": [
///             [
///                 "65485.47",
///                 "47.081829"
///             ]
///         ],
///         "ts": 1716863719031,
///         "u": 230704,
///         "seq": 1432604333,
///         "cts": 1716863718905
///     },
///     "retExtInfo": {},
///     "time": 1716863719382
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookL2 {
    #[serde(rename = "retCode")]
    pub ret_code: i64,
    #[serde(rename = "retMsg")]
    pub ret_msg: String,
    #[serde(rename = "result")]
    pub result: BybitOrderBookL2Result,
    #[serde(
        alias = "time",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc",
        default = "Utc::now"
    )]
    pub time: DateTime<Utc>,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitOrderBookL2Result {
    #[serde(alias = "s")]
    pub symbol: String,
    #[serde(alias = "a")]
    pub asks: Vec<BybitLevel>,
    #[serde(alias = "b")]
    pub bids: Vec<BybitLevel>,
    #[serde(
        alias = "ts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc",
        default = "Utc::now"
    )]
    pub time: DateTime<Utc>,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "seq")]
    pub sequence: u64,
    #[serde(
        alias = "cts",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc",
        default = "Utc::now"
    )]
    pub created_time: DateTime<Utc>,
}

impl From<BybitOrderBookL2> for OrderBook {
    fn from(snapshot: BybitOrderBookL2) -> Self {
        Self {
            last_update_time: Utc::now(),
            bids: OrderBookSide::new(Side::Buy, snapshot.result.bids),
            asks: OrderBookSide::new(Side::Sell, snapshot.result.asks),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use std::time::Duration;

        use barter_integration::de::datetime_utc_from_epoch_duration;

        use super::*;

        #[test]
        fn test_bybit_order_book_l2_snapshot() {
            struct TestCase {
                input: &'static str,
                expected: BybitOrderBookL2,
            }

            let res_time = datetime_utc_from_epoch_duration(Duration::from_millis(1716863719382));
            let time = datetime_utc_from_epoch_duration(Duration::from_millis(1716863719031));
            let created_time =
                datetime_utc_from_epoch_duration(Duration::from_millis(1716863718905));

            let tests = vec![TestCase {
                // TC0: valid BybitOrderBookL2
                input: r#"
                    {
                        "retCode": 0,
                        "retMsg": "OK",
                        "result": {
                            "s": "BTCUSDT",
                            "a": [
                                ["65557.7", "16.606555"]
                            ],
                            "b": [
                                ["65485.47", "47.081829"]
                            ],
                            "ts": 1716863719031,
                            "u": 230704,
                            "seq": 1432604333,
                            "cts": 1716863718905
                        },
                        "retExtInfo": {},
                        "time": 1716863719382
                    }
                    "#,
                expected: BybitOrderBookL2 {
                    ret_code: 0,
                    ret_msg: "OK".to_string(),
                    result: BybitOrderBookL2Result {
                        symbol: "BTCUSDT".to_string(),
                        asks: vec![BybitLevel {
                            price: 65557.7,
                            amount: 16.606555,
                        }],
                        bids: vec![BybitLevel {
                            price: 65485.47,
                            amount: 47.081829,
                        }],
                        time,
                        last_update_id: 230704,
                        sequence: 1432604333,
                        created_time,
                    },
                    time: res_time,
                },
            }];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    serde_json::from_str::<BybitOrderBookL2>(test.input).unwrap(),
                    test.expected,
                    "TC{} failed",
                    index
                );
            }
        }
    }
}

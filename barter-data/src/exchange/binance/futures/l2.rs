use super::super::book::BinanceLevel;
use crate::Identifier;
use barter_integration::model::SubscriptionId;
use serde::{Deserialize, Serialize};

/// [`BinanceFuturesUsd`](super::BinanceFuturesUsd) HTTP OrderBook L2 snapshot url.
///
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#order-book>
pub const HTTP_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT: &str = "https://fapi.binance.com/fapi/v1/depth";

/// [`BinanceFuturesUsd`](super::BinanceFuturesUsd) OrderBook Level2 deltas WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
/// ```json
/// {
///     "e": "depthUpdate",
///     "E": 123456789,
///     "T": 123456788,
///     "s": "BTCUSDT",
///     "U": 157,
///     "u": 160,
///     "pu": 149,
///     "b": [
///         ["0.0024", "10"]
///     ],
///     "a": [
///         ["0.0026", "100"]
///     ]
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceFuturesOrderBookL2Update {
    #[serde(
        alias = "s",
        deserialize_with = "super::super::book::l2::de_ob_l2_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "U")]
    pub first_update_id: u64,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "pu")]
    pub prev_last_update_id: u64,
    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceFuturesOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_de_binance_futures_order_book_l2_update() {
        let input = r#"
            {
                "e": "depthUpdate",
                "E": 123456789,
                "T": 123456788,
                "s": "BTCUSDT",
                "U": 157,
                "u": 160,
                "pu": 149,
                "b": [
                    [
                        "0.0024",
                        "10"
                    ]
                ],
                "a": [
                    [
                        "0.0026",
                        "100"
                    ]
                ]
            }
        "#;

        assert_eq!(
            serde_json::from_str::<BinanceFuturesOrderBookL2Update>(input).unwrap(),
            BinanceFuturesOrderBookL2Update {
                subscription_id: SubscriptionId::from("@depth@100ms|BTCUSDT"),
                first_update_id: 157,
                last_update_id: 160,
                prev_last_update_id: 149,
                bids: vec![BinanceLevel {
                    price: dec!(0.0024),
                    amount: dec!(10.0)
                },],
                asks: vec![BinanceLevel {
                    price: dec!(0.0026),
                    amount: dec!(100.0)
                },]
            }
        );
    }
}

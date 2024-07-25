use crate::subscription::book::Level;
use serde::{Deserialize, Serialize};

/// Level 1 OrderBook types (top of book).
pub mod l1;

/// Level 2 OrderBook types (top of book).
pub mod l2;

/// [`Bybit`](super::Bybit) OrderBook level.
///
/// #### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook>
/// ```json
/// ["4.00000200", "12.00000000"]
/// ```
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLevel {
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
}

impl From<BybitLevel> for Level {
    fn from(level: BybitLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_bybit_level() {
            let input = r#"["4.00000200", "12.00000000"]"#;
            assert_eq!(
                serde_json::from_str::<BybitLevel>(input).unwrap(),
                BybitLevel {
                    price: 4.00000200,
                    amount: 12.0
                },
            )
        }
    }
}

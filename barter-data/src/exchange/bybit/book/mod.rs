use crate::books::Level;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Level 1 OrderBook types.
pub mod l1;

/// Level 2 OrderBook types.
pub mod l2;

/// [`Bybit`](super::Bybit) OrderBook level.
///
/// #### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook#response-parameters>
/// ```json
/// ["16493.50", "0.006"]
/// ```
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitLevel {
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
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
        use rust_decimal_macros::dec;

        #[test]
        fn test_bybit_level() {
            let input = r#"["16493.50", "0.006"]"#;
            assert_eq!(
                serde_json::from_str::<BybitLevel>(input).unwrap(),
                BybitLevel {
                    price: dec!(16493.50),
                    amount: dec!(0.006)
                },
            )
        }
    }
}

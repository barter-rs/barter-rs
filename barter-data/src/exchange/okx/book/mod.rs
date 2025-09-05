use crate::books::Level;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Level 1 OrderBook types (top of books).
pub mod l1;

/// Level 2 OrderBook types.
pub mod l2;

/// [`Okx`](super::Okx) OrderBook level.
///
/// #### Raw Payload Examples
/// See docs: <https://www.okx.com/docs-v5/en/#order-book-trading-market-data-ws-order-book-channel>
/// ```json
/// ["411.8", "10.2"]
/// ```
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct OkxLevel {
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

impl From<OkxLevel> for Level {
    fn from(level: OkxLevel) -> Self {
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
        fn test_okx_level() {
            let input = r#"["411.8", "10.2"]"#;
            assert_eq!(
                serde_json::from_str::<OkxLevel>(input).unwrap(),
                OkxLevel {
                    price: dec!(411.8),
                    amount: dec!(10.2)
                },
            )
        }
    }
}
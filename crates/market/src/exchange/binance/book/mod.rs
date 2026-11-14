use crate::books::Level;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Level 1 OrderBook types (top of books).
pub mod l1;

/// Level 2 OrderBook types.
pub mod l2;

/// [`Binance`](super::Binance) OrderBook level.
///
/// #### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/futures/en/#partial-book-depth-streams>
/// ```json
/// ["4.00000200", "12.00000000"]
/// ```
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceLevel {
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
}

impl From<BinanceLevel> for Level {
    fn from(level: BinanceLevel) -> Self {
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
        fn test_binance_level() {
            let input = r#"["4.00000200", "12.00000000"]"#;
            assert_eq!(
                serde_json::from_str::<BinanceLevel>(input).unwrap(),
                BinanceLevel {
                    price: dec!(4.00000200),
                    amount: dec!(12.0)
                },
            )
        }
    }
}

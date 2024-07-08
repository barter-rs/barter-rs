use crate::subscription::book::Level;
use serde::{Deserialize, Serialize};

/// Level 1 OrderBook types (top of book).
pub mod l1;

/// Level 2 OrderBook types (top of book).
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
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,
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

        #[test]
        fn test_binance_level() {
            let input = r#"["4.00000200", "12.00000000"]"#;
            assert_eq!(
                serde_json::from_str::<BinanceLevel>(input).unwrap(),
                BinanceLevel {
                    price: 4.00000200,
                    amount: 12.0
                },
            )
        }
    }
}

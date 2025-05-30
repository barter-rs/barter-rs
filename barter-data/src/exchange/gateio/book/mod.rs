use crate::books::Level;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Level 1 OrderBook types (top of books).
pub mod l1;

/// Level 2 OrderBook types.
pub mod l2;

/// [`Gateio`](super::Gateio) OrderBook level.
///
/// #### Raw Payload Examples
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#limited-level-full-order-book-snapshot>
///
/// ```json
/// ["16493.50", "0.006"]
/// ```
#[derive(Debug, Deserialize, Clone, Serialize, PartialOrd, PartialEq)]
struct GateioLevel {
    #[serde(with = "rust_decimal::serde::str")]
    price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    amount: Decimal,
}

impl From<GateioLevel> for Level {
    fn from(level: GateioLevel) -> Self {
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
        fn test_gateio_level() {
            let input = r#"["4.00000200", "12.00000000"]"#;
            assert_eq!(
                serde_json::from_str::<GateioLevel>(input).unwrap(),
                GateioLevel {
                    price: dec!(4.00000200),
                    amount: dec!(12.0)
                },
            )
        }
    }
}

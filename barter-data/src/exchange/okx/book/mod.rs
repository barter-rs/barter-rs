use crate::subscription::book::Level;
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

/// [`Okx`](super::Okx) OrderBookL1 types
pub mod l1;

/// [`Okx`](super::Okx) OrderBookL2 types
pub mod l2;

/// [`Okx`](super::Okx) levels
///
/// From OKX docs:
/// > An example of the array of asks and bids values: ["411.8", "10", "0", "4"]
/// > "411.8" is the depth price
/// > "10" is the quantity at the price (number of contracts for derivatives, quantity in base currency for Spot and Spot Margin)
/// > "0" is part of a deprecated feature and it is always "0"
/// > "4" is the number of orders at the price.
///
/// Note:
/// We are only interested in the depth and quantity
///
/// See docs: <https://www.okx.com/docs-v5/en/#order-book-trading-market-data-ws-order-book-channel>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
pub struct OkxLevel {
    pub price: f64,
    pub amount: f64,
}

impl<'de> Deserialize<'de> for OkxLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OkxLevelVisitor;

        impl<'de> Visitor<'de> for OkxLevelVisitor {
            type Value = OkxLevel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("array where the first two elements represent price and amount")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<OkxLevel, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let price: f64 = seq
                    .next_element::<&str>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?
                    .parse()
                    .map_err(serde::de::Error::custom)?;
                let amount: f64 = seq
                    .next_element::<&str>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?
                    .parse()
                    .map_err(serde::de::Error::custom)?;

                // Ignore remaining elements
                while seq.next_element::<serde_json::Value>()?.is_some() {}

                Ok(OkxLevel { price, amount })
            }
        }

        deserializer.deserialize_seq(OkxLevelVisitor)
    }
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

        #[test]
        fn test_okx_level() {
            let input = r#"["4.00000200", "12.00000000", "0", "42.42"]"#;
            assert_eq!(
                serde_json::from_str::<OkxLevel>(input).unwrap(),
                OkxLevel {
                    price: 4.00000200,
                    amount: 12.0
                },
            )
        }
    }
}

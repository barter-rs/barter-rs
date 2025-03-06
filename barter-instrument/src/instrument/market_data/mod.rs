use crate::asset::name::AssetNameInternal;
use kind::MarketDataInstrumentKind;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub mod kind;

/// Barter representation of an `MarketDataInstrument`. Used to uniquely identify a `base_quote`
/// pair, and it's associated instrument type.
///
/// eg/ MarketDataInstrument { base: "btc", quote: "usdt", kind: Spot }
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MarketDataInstrument {
    pub base: AssetNameInternal,
    pub quote: AssetNameInternal,
    #[serde(rename = "instrument_kind")]
    pub kind: MarketDataInstrumentKind,
}

impl Display for MarketDataInstrument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}_{}", self.base, self.quote, self.kind)
    }
}

impl<S> From<(S, S, MarketDataInstrumentKind)> for MarketDataInstrument
where
    S: Into<AssetNameInternal>,
{
    fn from((base, quote, kind): (S, S, MarketDataInstrumentKind)) -> Self {
        Self {
            base: base.into(),
            quote: quote.into(),
            kind,
        }
    }
}

impl MarketDataInstrument {
    /// Constructs a new [`MarketDataInstrument`] using the provided configuration.
    pub fn new<S>(base: S, quote: S, kind: MarketDataInstrumentKind) -> Self
    where
        S: Into<AssetNameInternal>,
    {
        Self {
            base: base.into(),
            quote: quote.into(),
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::instrument::{
        kind::option::{OptionExercise, OptionKind},
        market_data::{
            MarketDataInstrument,
            kind::{MarketDataFutureContract, MarketDataInstrumentKind, MarketDataOptionContract},
        },
    };
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn test_de_instrument() {
        struct TestCase {
            input: &'static str,
            expected: Result<MarketDataInstrument, serde_json::Error>,
        }

        let cases = vec![
            TestCase {
                // TC0: Valid Spot
                input: r#"{"base": "btc", "quote": "usd", "instrument_kind": "spot" }"#,
                expected: Ok(MarketDataInstrument::from((
                    "btc",
                    "usd",
                    MarketDataInstrumentKind::Spot,
                ))),
            },
            TestCase {
                // TC1: Valid Future
                input: r#"{
                    "base": "btc",
                    "quote": "usd",
                    "instrument_kind": {"future": {"expiry": 1703980800000}}
                }"#,
                expected: Ok(MarketDataInstrument::new(
                    "btc",
                    "usd",
                    MarketDataInstrumentKind::Future(MarketDataFutureContract {
                        expiry: Utc.timestamp_millis_opt(1703980800000).unwrap(),
                    }),
                )),
            },
            TestCase {
                // TC2: Valid FuturePerpetual
                input: r#"{"base": "btc", "quote": "usd", "instrument_kind": "perpetual" }"#,
                expected: Ok(MarketDataInstrument::from((
                    "btc",
                    "usd",
                    MarketDataInstrumentKind::Perpetual,
                ))),
            },
            TestCase {
                // TC3: Valid Option Call American
                input: r#"{
                    "base": "btc",
                    "quote": "usd",
                    "instrument_kind": {
                        "option": {
                            "kind": "CALL",
                            "exercise": "American",
                            "expiry": 1703980800000,
                            "strike": 50000
                        }
                    }
                }"#,
                expected: Ok(MarketDataInstrument::from((
                    "btc",
                    "usd",
                    MarketDataInstrumentKind::Option(MarketDataOptionContract {
                        kind: OptionKind::Call,
                        exercise: OptionExercise::American,
                        expiry: Utc.timestamp_millis_opt(1703980800000).unwrap(),
                        strike: dec!(50000),
                    }),
                ))),
            },
            TestCase {
                // TC4: Valid Option Put Bermudan
                input: r#"{
                    "base": "btc",
                    "quote": "usd",
                    "instrument_kind": {
                        "option": {
                            "kind": "Put",
                            "exercise": "BERMUDAN",
                            "expiry": 1703980800000,
                            "strike": 50000
                        }
                    }
                }"#,
                expected: Ok(MarketDataInstrument::from((
                    "btc",
                    "usd",
                    MarketDataInstrumentKind::Option(MarketDataOptionContract {
                        kind: OptionKind::Put,
                        exercise: OptionExercise::Bermudan,
                        expiry: Utc.timestamp_millis_opt(1703980800000).unwrap(),
                        strike: dec!(50000.0),
                    }),
                ))),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<MarketDataInstrument>(test.input);

            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!(
                        "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                    );
                }
            }
        }
    }
}

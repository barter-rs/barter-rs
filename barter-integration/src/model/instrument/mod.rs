use crate::model::instrument::{kind::InstrumentKind, symbol::Symbol};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub mod kind;
pub mod symbol;

/// Barter representation of an `Instrument`. Used to uniquely identify a `base_quote` pair, and it's
/// associated instrument type.
///
/// eg/ Instrument { base: "btc", quote: "usdt", kind: Spot }
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Instrument {
    pub base: Symbol,
    pub quote: Symbol,
    #[serde(rename = "instrument_kind")]
    pub kind: InstrumentKind,
}

impl Display for Instrument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}_{}, {})", self.base, self.quote, self.kind)
    }
}

impl<S> From<(S, S, InstrumentKind)> for Instrument
where
    S: Into<Symbol>,
{
    fn from((base, quote, kind): (S, S, InstrumentKind)) -> Self {
        Self {
            base: base.into(),
            quote: quote.into(),
            kind,
        }
    }
}

impl Instrument {
    /// Constructs a new [`Instrument`] using the provided configuration.
    pub fn new<S>(base: S, quote: S, kind: InstrumentKind) -> Self
    where
        S: Into<Symbol>,
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
    use super::*;
    use crate::model::instrument::kind::{
        FutureContract, InstrumentKind, OptionContract, OptionExercise, OptionKind,
    };
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn test_de_instrument() {
        struct TestCase {
            input: &'static str,
            expected: Result<Instrument, serde_json::Error>,
        }

        let cases = vec![
            TestCase {
                // TC0: Valid Spot
                input: r#"{"base": "btc", "quote": "usd", "instrument_kind": "spot" }"#,
                expected: Ok(Instrument::from(("btc", "usd", InstrumentKind::Spot))),
            },
            TestCase {
                // TC1: Valid Future
                input: r#"{
                    "base": "btc",
                    "quote": "usd",
                    "instrument_kind": {"future": {"expiry": 1703980800000}}
                }"#,
                expected: Ok(Instrument::new(
                    "btc",
                    "usd",
                    InstrumentKind::Future(FutureContract {
                        expiry: Utc.timestamp_millis_opt(1703980800000).unwrap(),
                    }),
                )),
            },
            TestCase {
                // TC2: Valid FuturePerpetual
                input: r#"{"base": "btc", "quote": "usd", "instrument_kind": "perpetual" }"#,
                expected: Ok(Instrument::from(("btc", "usd", InstrumentKind::Perpetual))),
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
                expected: Ok(Instrument::from((
                    "btc",
                    "usd",
                    InstrumentKind::Option(OptionContract {
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
                expected: Ok(Instrument::from((
                    "btc",
                    "usd",
                    InstrumentKind::Option(OptionContract {
                        kind: OptionKind::Put,
                        exercise: OptionExercise::Bermudan,
                        expiry: Utc.timestamp_millis_opt(1703980800000).unwrap(),
                        strike: dec!(50000.0),
                    }),
                ))),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<Instrument>(test.input);

            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                }
            }
        }
    }
}

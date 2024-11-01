use crate::{
    asset::name::AssetNameInternal,
    exchange::ExchangeId,
    instrument::market_data::{kind::MarketDataInstrumentKind, MarketDataInstrument},
};
use serde::{Deserialize, Serialize};
use smol_str::{format_smolstr, SmolStr, StrExt};
use std::fmt::{Debug, Display, Formatter};

/// Represents a unique combination of an [`Exchange`] & an [`MarketDataInstrument`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Market<InstrumentKey = MarketDataInstrument> {
    pub exchange: ExchangeId,
    #[serde(flatten)]
    pub instrument: InstrumentKey,
}

impl<E, I, InstrumentId> From<(E, I)> for Market<InstrumentId>
where
    E: Into<ExchangeId>,
    I: Into<InstrumentId>,
{
    fn from((exchange, instrument): (E, I)) -> Self {
        Self::new(exchange, instrument)
    }
}

impl<E, S> From<(E, S, S, MarketDataInstrumentKind)> for Market<MarketDataInstrument>
where
    E: Into<ExchangeId>,
    S: Into<AssetNameInternal>,
{
    fn from((exchange, base, quote, instrument_kind): (E, S, S, MarketDataInstrumentKind)) -> Self {
        Self::new(exchange, (base, quote, instrument_kind))
    }
}

impl<InstrumentId> Market<InstrumentId> {
    /// Constructs a new [`Market`] using the provided [`Exchange`] & [`MarketDataInstrument`].
    pub fn new<E, I>(exchange: E, instrument: I) -> Self
    where
        E: Into<ExchangeId>,
        I: Into<InstrumentId>,
    {
        Self {
            exchange: exchange.into(),
            instrument: instrument.into(),
        }
    }
}

/// Barter new type representing a unique `String` identifier for a [`Market`], where a [`Market`]
/// represents an [`MarketDataInstrument`] being traded on an [`Exchange`].
///
/// eg/ binance_(btc_spot, future_perpetual)
/// eg/ ftx_btc_usdt_future_perpetual
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct MarketId(pub SmolStr);

impl Debug for MarketId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for MarketId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for MarketId {
    fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        SmolStr::deserialize(deserializer).map(MarketId)
    }
}

impl<InstrumentId: Display> From<&Market<InstrumentId>> for MarketId {
    fn from(value: &Market<InstrumentId>) -> Self {
        Self(format_smolstr!("{}_{}", value.exchange, value.instrument).to_lowercase_smolstr())
    }
}

impl MarketId {
    /// Construct a unique `String` [`MarketId`] identifier for a [`Market`], where a [`Market`]
    /// represents an [`MarketDataInstrument`] being traded on an [`Exchange`].
    pub fn new(exchange: ExchangeId, instrument: &MarketDataInstrument) -> Self {
        Self(
            format_smolstr!(
                "{}_{}_{}_{}",
                exchange,
                instrument.base,
                instrument.quote,
                instrument.kind
            )
            .to_lowercase_smolstr(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Error;

    #[test]
    fn test_de_market() {
        struct TestCase {
            input: &'static str,
            expected: Result<Market, serde_json::Error>,
        }

        let cases = vec![
            TestCase {
                // TC0: Valid Binance btc_usd Spot Market
                input: r##"{ "exchange": "binance_spot", "base": "btc", "quote": "usd", "instrument_kind": "spot" }"##,
                expected: Ok(Market {
                    exchange: ExchangeId::BinanceSpot,
                    instrument: MarketDataInstrument::from((
                        "btc",
                        "usd",
                        MarketDataInstrumentKind::Spot,
                    )),
                }),
            },
            TestCase {
                // TC1: Valid Ftx btc_usd FuturePerpetual Market
                input: r##"{ "exchange": "other", "base": "btc", "quote": "usd", "instrument_kind": "perpetual" }"##,
                expected: Ok(Market {
                    exchange: ExchangeId::Other,
                    instrument: MarketDataInstrument::from((
                        "btc",
                        "usd",
                        MarketDataInstrumentKind::Perpetual,
                    )),
                }),
            },
            TestCase {
                // TC3: Invalid Market w/ numeric exchange
                input: r##"{ "exchange": 100, "base": "btc", "quote": "usd", "instrument_kind": "perpetual" }"##,
                expected: Err(serde_json::Error::custom("")),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<Market>(test.input);

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

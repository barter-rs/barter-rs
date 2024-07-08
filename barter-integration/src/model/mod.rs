use crate::model::instrument::{kind::InstrumentKind, symbol::Symbol, Instrument};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
};

/// [`Instrument`] related data structures.
///
/// eg/ `Instrument`, `InstrumentKind`, `OptionContract`, `Symbol`, etc.
pub mod instrument;

/// Represents a unique combination of an [`Exchange`] & an [`Instrument`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct Market<InstrumentId = Instrument> {
    pub exchange: Exchange,
    #[serde(flatten)]
    pub instrument: InstrumentId,
}

impl<E, I, InstrumentId> From<(E, I)> for Market<InstrumentId>
where
    E: Into<Exchange>,
    I: Into<InstrumentId>,
{
    fn from((exchange, instrument): (E, I)) -> Self {
        Self::new(exchange, instrument)
    }
}

impl<E, S> From<(E, S, S, InstrumentKind)> for Market<Instrument>
where
    E: Into<Exchange>,
    S: Into<Symbol>,
{
    fn from((exchange, base, quote, instrument_kind): (E, S, S, InstrumentKind)) -> Self {
        Self::new(exchange, (base, quote, instrument_kind))
    }
}

impl<InstrumentId> Market<InstrumentId> {
    /// Constructs a new [`Market`] using the provided [`Exchange`] & [`Instrument`].
    pub fn new<E, I>(exchange: E, instrument: I) -> Self
    where
        E: Into<Exchange>,
        I: Into<InstrumentId>,
    {
        Self {
            exchange: exchange.into(),
            instrument: instrument.into(),
        }
    }
}

/// Barter new type representing a unique `String` identifier for a [`Market`], where a [`Market`]
/// represents an [`Instrument`] being traded on an [`Exchange`].
///
/// eg/ binance_(btc_spot, future_perpetual)
/// eg/ ftx_btc_usdt_future_perpetual
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub struct MarketId(pub String);

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
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(MarketId)
    }
}

impl<InstrumentId: Display> From<&Market<InstrumentId>> for MarketId {
    fn from(value: &Market<InstrumentId>) -> Self {
        Self(format!("{}_{}", value.exchange, value.instrument).to_lowercase())
    }
}

impl MarketId {
    /// Construct a unique `String` [`MarketId`] identifier for a [`Market`], where a [`Market`]
    /// represents an [`Instrument`] being traded on an [`Exchange`].
    pub fn new(exchange: &Exchange, instrument: &Instrument) -> Self {
        Self(
            format!(
                "{}_{}_{}_{}",
                exchange, instrument.base, instrument.quote, instrument.kind
            )
            .to_lowercase(),
        )
    }
}

/// Barter representation of an [`Exchange`]'s name.
///
/// eg/ Exchange("binance_spot"), Exchange("bitfinex"), Exchange("gateio_spot"), etc.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Exchange(Cow<'static, str>);

impl<E> From<E> for Exchange
where
    E: Into<Cow<'static, str>>,
{
    fn from(exchange: E) -> Self {
        Exchange(exchange.into())
    }
}

impl Debug for Exchange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Exchange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// New type representing a unique `String` identifier for a stream that has been subscribed to.
/// This is used to identify data structures received over the socket.
///
/// For example, `Barter-Data` uses this identifier to associate received data structures from the
/// exchange with the original `Barter-Data` `Subscription` that was actioned over the socket.
///
/// Note: Each exchange will require the use of different `String` identifiers depending on the
/// data structures they send.
///
/// eg/ [`SubscriptionId`] of an `FtxTrade` is "{BASE}/{QUOTE}" (ie/ market).
/// eg/ [`SubscriptionId`] of a `BinanceTrade` is "{base}{symbol}@trade" (ie/ channel).
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct SubscriptionId(pub String);

impl Debug for SubscriptionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for SubscriptionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SubscriptionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<S> From<S> for SubscriptionId
where
    S: Into<String>,
{
    fn from(input: S) -> Self {
        Self(input.into())
    }
}

/// [`Side`] of a trade or position - Buy or Sell.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum Side {
    #[serde(alias = "buy", alias = "BUY", alias = "b")]
    Buy,
    #[serde(alias = "sell", alias = "SELL", alias = "s")]
    Sell,
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Side::Buy => "buy",
                Side::Sell => "sell",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::instrument::{kind::InstrumentKind, Instrument};
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
                input: r##"{ "exchange": "binance", "base": "btc", "quote": "usd", "instrument_kind": "spot" }"##,
                expected: Ok(Market {
                    exchange: Exchange::from("binance"),
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Spot)),
                }),
            },
            TestCase {
                // TC1: Valid Ftx btc_usd FuturePerpetual Market
                input: r##"{ "exchange": "ftx_old", "base": "btc", "quote": "usd", "instrument_kind": "perpetual" }"##,
                expected: Ok(Market {
                    exchange: Exchange::from("ftx_old"),
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::Perpetual)),
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

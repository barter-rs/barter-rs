use crate::{
    exchange::{Connector, ExchangeId},
    instrument::{InstrumentData, KeyedInstrument},
};
use barter_integration::{
    error::SocketError,
    model::{
        instrument::{kind::InstrumentKind, symbol::Symbol, Instrument},
        SubscriptionId,
    },
    protocol::websocket::WsMessage,
    Validator,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
};

/// OrderBook [`SubscriptionKind`]s and the associated Barter output data models.
pub mod book;

/// Candle [`SubscriptionKind`] and the associated Barter output data model.
pub mod candle;

/// Liquidation [`SubscriptionKind`] and the associated Barter output data model.
pub mod liquidation;

/// Public trade [`SubscriptionKind`] and the associated Barter output data model.
pub mod trade;

/// Defines the type of a [`Subscription`], and the output [`Self::Event`] that it yields.
pub trait SubscriptionKind
where
    Self: Debug + Clone,
{
    type Event: Debug;
}

/// Barter [`Subscription`] used to subscribe to a [`SubscriptionKind`] for a particular exchange
/// [`Instrument`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Subscription<Exchange = ExchangeId, Inst = Instrument, Kind = SubKind> {
    pub exchange: Exchange,
    #[serde(flatten)]
    pub instrument: Inst,
    #[serde(alias = "type")]
    pub kind: Kind,
}

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize, Display,
)]
pub enum SubKind {
    PublicTrades,
    OrderBooksL1,
    OrderBooksL2,
    OrderBooksL3,
    Liquidations,
    Candles,
}

impl<Exchange, Instrument, Kind> Display for Subscription<Exchange, Instrument, Kind>
where
    Exchange: Display,
    Instrument: Display,
    Kind: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}{}", self.exchange, self.kind, self.instrument)
    }
}

impl<Exchange, S, Kind> From<(Exchange, S, S, InstrumentKind, Kind)>
    for Subscription<Exchange, Instrument, Kind>
where
    S: Into<Symbol>,
{
    fn from(
        (exchange, base, quote, instrument_kind, kind): (Exchange, S, S, InstrumentKind, Kind),
    ) -> Self {
        Self::new(exchange, (base, quote, instrument_kind), kind)
    }
}

impl<InstrumentId, Exchange, S, Kind> From<(InstrumentId, Exchange, S, S, InstrumentKind, Kind)>
    for Subscription<Exchange, KeyedInstrument<InstrumentId>, Kind>
where
    S: Into<Symbol>,
{
    fn from(
        (instrument_id, exchange, base, quote, instrument_kind, kind): (
            InstrumentId,
            Exchange,
            S,
            S,
            InstrumentKind,
            Kind,
        ),
    ) -> Self {
        let instrument = KeyedInstrument::new(instrument_id, (base, quote, instrument_kind).into());

        Self::new(exchange, instrument, kind)
    }
}

impl<Exchange, I, Instrument, Kind> From<(Exchange, I, Kind)>
    for Subscription<Exchange, Instrument, Kind>
where
    I: Into<Instrument>,
{
    fn from((exchange, instrument, kind): (Exchange, I, Kind)) -> Self {
        Self::new(exchange, instrument, kind)
    }
}

impl<Instrument, Exchange, Kind> Subscription<Exchange, Instrument, Kind> {
    /// Constructs a new [`Subscription`] using the provided configuration.
    pub fn new<I>(exchange: Exchange, instrument: I, kind: Kind) -> Self
    where
        I: Into<Instrument>,
    {
        Self {
            exchange,
            instrument: instrument.into(),
            kind,
        }
    }
}

impl<Exchange, Kind> Validator for &Subscription<Exchange, Instrument, Kind>
where
    Exchange: Connector,
{
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        // Determine ExchangeId associated with this Subscription
        let exchange = Exchange::ID;

        // Validate the Exchange supports the Subscription InstrumentKind
        if exchange.supports_instrument_kind(self.instrument.kind) {
            Ok(self)
        } else {
            Err(SocketError::Unsupported {
                entity: exchange.as_str(),
                item: self.instrument.kind.to_string(),
            })
        }
    }
}

impl<Instrument> Validator for Subscription<ExchangeId, Instrument, SubKind>
where
    Instrument: InstrumentData,
{
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        // Validate the Exchange supports the Subscription InstrumentKind
        if self.exchange.supports(self.instrument.kind(), self.kind) {
            Ok(self)
        } else {
            Err(SocketError::Unsupported {
                entity: self.exchange.as_str(),
                item: self.instrument.kind().to_string(),
            })
        }
    }
}

/// Metadata generated from a collection of Barter [`Subscription`]s, including the exchange
/// specific subscription payloads that are sent to the exchange.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct SubscriptionMeta<InstrumentId> {
    /// `HashMap` containing the mapping between a [`SubscriptionId`] and
    /// it's associated Barter [`Instrument`].
    pub instrument_map: Map<InstrumentId>,
    /// Collection of [`WsMessage`]s containing exchange specific subscription payloads to be sent.
    pub subscriptions: Vec<WsMessage>,
}

/// New type`HashMap` that maps a [`SubscriptionId`] to some associated type `T`.
///
/// Used by [`ExchangeTransformer`](crate::transformer::ExchangeTransformer)s to identify the
/// Barter [`Instrument`] associated with incoming exchange messages.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Map<T>(pub HashMap<SubscriptionId, T>);

impl<T> FromIterator<(SubscriptionId, T)> for Map<T> {
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = (SubscriptionId, T)>,
    {
        Self(iter.into_iter().collect::<HashMap<SubscriptionId, T>>())
    }
}

impl<T> Map<T> {
    /// Find the `InstrumentId` associated with the provided [`SubscriptionId`].
    pub fn find<SubId>(&self, id: &SubId) -> Result<&T, SocketError>
    where
        SubscriptionId: Borrow<SubId>,
        SubId: AsRef<str> + Hash + Eq + ?Sized,
    {
        self.0
            .get(id)
            .ok_or_else(|| SocketError::Unidentifiable(SubscriptionId(id.as_ref().to_string())))
    }

    /// Find the mutable reference to `T` associated with the provided [`SubscriptionId`].
    pub fn find_mut<SubId>(&mut self, id: &SubId) -> Result<&mut T, SocketError>
    where
        SubscriptionId: Borrow<SubId>,
        SubId: AsRef<str> + Hash + Eq + ?Sized,
    {
        self.0
            .get_mut(id)
            .ok_or_else(|| SocketError::Unidentifiable(SubscriptionId(id.as_ref().to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod subscription {
        use super::*;
        use crate::{
            exchange::{coinbase::Coinbase, okx::Okx},
            subscription::trade::PublicTrades,
        };
        use barter_integration::model::instrument::kind::InstrumentKind;

        mod de {
            use super::*;
            use crate::{
                exchange::{
                    binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
                    gateio::perpetual::GateioPerpetualsUsd,
                    okx::Okx,
                },
                subscription::{book::OrderBooksL2, trade::PublicTrades},
            };

            #[test]
            fn test_subscription_okx_spot_public_trades() {
                let input = r#"
                {
                    "exchange": "okx",
                    "base": "btc",
                    "quote": "usdt",
                    "instrument_kind": "spot",
                    "kind": "public_trades"
                }
                "#;

                serde_json::from_str::<Subscription<Okx, Instrument, PublicTrades>>(input).unwrap();
            }

            #[test]
            fn test_subscription_binance_spot_public_trades() {
                let input = r#"
                {
                    "exchange": "binance_spot",
                    "base": "btc",
                    "quote": "usdt",
                    "instrument_kind": "spot",
                    "kind": "public_trades"
                }
                "#;

                serde_json::from_str::<Subscription<BinanceSpot, Instrument, PublicTrades>>(input)
                    .unwrap();
            }

            #[test]
            fn test_subscription_binance_futures_usd_order_books_l2() {
                let input = r#"
                {
                    "exchange": "binance_futures_usd",
                    "base": "btc",
                    "quote": "usdt",
                    "instrument_kind": "perpetual",
                    "kind": "order_books_l2"
                }
                "#;

                serde_json::from_str::<Subscription<BinanceFuturesUsd, Instrument, OrderBooksL2>>(
                    input,
                )
                .unwrap();
            }

            #[test]
            fn subscription_gateio_futures_usd_public_trades() {
                let input = r#"
                {
                    "exchange": "gateio_perpetuals_usd",
                    "base": "btc",
                    "quote": "usdt",
                    "instrument_kind": "perpetual",
                    "kind": "public_trades"
                }
                "#;

                serde_json::from_str::<Subscription<GateioPerpetualsUsd, Instrument, PublicTrades>>(input)
                    .unwrap();
            }
        }

        #[test]
        fn test_validate_bitfinex_public_trades() {
            struct TestCase {
                input: Subscription<Coinbase, Instrument, PublicTrades>,
                expected: Result<Subscription<Coinbase, Instrument, PublicTrades>, SocketError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Valid Coinbase Spot PublicTrades subscription
                    input: Subscription::from((
                        Coinbase,
                        "base",
                        "quote",
                        InstrumentKind::Spot,
                        PublicTrades,
                    )),
                    expected: Ok(Subscription::from((
                        Coinbase,
                        "base",
                        "quote",
                        InstrumentKind::Spot,
                        PublicTrades,
                    ))),
                },
                TestCase {
                    // TC1: Invalid Coinbase FuturePerpetual PublicTrades subscription
                    input: Subscription::from((
                        Coinbase,
                        "base",
                        "quote",
                        InstrumentKind::Perpetual,
                        PublicTrades,
                    )),
                    expected: Err(SocketError::Unsupported {
                        entity: "",
                        item: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = test.input.validate();
                match (actual, &test.expected) {
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

        #[test]
        fn test_validate_okx_public_trades() {
            struct TestCase {
                input: Subscription<Okx, Instrument, PublicTrades>,
                expected: Result<Subscription<Okx, Instrument, PublicTrades>, SocketError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Valid Okx Spot PublicTrades subscription
                    input: Subscription::from((
                        Okx,
                        "base",
                        "quote",
                        InstrumentKind::Spot,
                        PublicTrades,
                    )),
                    expected: Ok(Subscription::from((
                        Okx,
                        "base",
                        "quote",
                        InstrumentKind::Spot,
                        PublicTrades,
                    ))),
                },
                TestCase {
                    // TC1: Valid Okx FuturePerpetual PublicTrades subscription
                    input: Subscription::from((
                        Okx,
                        "base",
                        "quote",
                        InstrumentKind::Perpetual,
                        PublicTrades,
                    )),
                    expected: Ok(Subscription::from((
                        Okx,
                        "base",
                        "quote",
                        InstrumentKind::Perpetual,
                        PublicTrades,
                    ))),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = test.input.validate();
                match (actual, &test.expected) {
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

    mod instrument_map {
        use super::*;
        use barter_integration::model::instrument::{kind::InstrumentKind, Instrument};

        #[test]
        fn test_find_instrument() {
            // Initialise SubscriptionId-InstrumentId HashMap
            let ids = Map(HashMap::from_iter([(
                SubscriptionId::from("present"),
                Instrument::from(("base", "quote", InstrumentKind::Spot)),
            )]));

            struct TestCase {
                input: SubscriptionId,
                expected: Result<Instrument, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: SubscriptionId (channel) is present in the HashMap
                    input: SubscriptionId::from("present"),
                    expected: Ok(Instrument::from(("base", "quote", InstrumentKind::Spot))),
                },
                TestCase {
                    // TC1: SubscriptionId (channel) is not present in the HashMap
                    input: SubscriptionId::from("not present"),
                    expected: Err(SocketError::Unidentifiable(SubscriptionId::from(
                        "not present",
                    ))),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = ids.find(&test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(*actual, expected, "TC{} failed", index)
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
}

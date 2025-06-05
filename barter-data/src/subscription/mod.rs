use crate::{exchange::Connector, instrument::InstrumentData};
use barter_instrument::{
    Keyed,
    asset::name::AssetNameInternal,
    exchange::ExchangeId,
    instrument::market_data::{MarketDataInstrument, kind::MarketDataInstrumentKind},
};
use barter_integration::{
    Validator, error::SocketError, protocol::websocket::WsMessage, subscription::SubscriptionId,
};
use derive_more::Display;
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use smol_str::{ToSmolStr, format_smolstr};
use std::{borrow::Borrow, fmt::Debug, hash::Hash};

/// OrderBook [`SubscriptionKind`]s and the associated Barter output data models.
pub mod book;

/// Candle [`SubscriptionKind`] and the associated Barter output data model.
pub mod candle;

/// Liquidation [`SubscriptionKind`] and the associated Barter output data model.
pub mod liquidation;

/// Public trade [`SubscriptionKind`] and the associated Barter output data model.
pub mod trade;

/// Defines kind of a [`Subscription`], and the output [`Self::Event`] that it yields.
pub trait SubscriptionKind
where
    Self: Debug + Clone,
{
    type Event: Debug;
    fn as_str(&self) -> &'static str;
}

/// Barter [`Subscription`] used to subscribe to a [`SubscriptionKind`] for a particular exchange
/// [`MarketDataInstrument`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Subscription<Exchange = ExchangeId, Inst = MarketDataInstrument, Kind = SubKind> {
    pub exchange: Exchange,
    #[serde(flatten)]
    pub instrument: Inst,
    #[serde(alias = "type")]
    pub kind: Kind,
}

pub fn display_subscriptions_without_exchange<Exchange, Instrument, Kind>(
    subscriptions: &[Subscription<Exchange, Instrument, Kind>],
) -> String
where
    Instrument: std::fmt::Display,
    Kind: std::fmt::Display,
{
    subscriptions
        .iter()
        .map(
            |Subscription {
                 exchange: _,
                 instrument,
                 kind,
             }| { format_smolstr!("({instrument}, {kind})") },
        )
        .collect::<Vec<_>>()
        .join(",")
}

impl<Exchange, Instrument, Kind> std::fmt::Display for Subscription<Exchange, Instrument, Kind>
where
    Exchange: std::fmt::Display,
    Instrument: std::fmt::Display,
    Kind: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}|{}|{})", self.exchange, self.kind, self.instrument)
    }
}

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display, Deserialize, Serialize,
)]
pub enum SubKind {
    PublicTrades,
    OrderBooksL1,
    OrderBooksL2,
    OrderBooksL3,
    Liquidations,
    Candles,
}

impl<Exchange, S, Kind> From<(Exchange, S, S, MarketDataInstrumentKind, Kind)>
    for Subscription<Exchange, MarketDataInstrument, Kind>
where
    S: Into<AssetNameInternal>,
{
    fn from(
        (exchange, base, quote, instrument_kind, kind): (
            Exchange,
            S,
            S,
            MarketDataInstrumentKind,
            Kind,
        ),
    ) -> Self {
        Self::new(exchange, (base, quote, instrument_kind), kind)
    }
}

impl<InstrumentKey, Exchange, S, Kind>
    From<(
        InstrumentKey,
        Exchange,
        S,
        S,
        MarketDataInstrumentKind,
        Kind,
    )> for Subscription<Exchange, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
where
    S: Into<AssetNameInternal>,
{
    fn from(
        (instrument_id, exchange, base, quote, instrument_kind, kind): (
            InstrumentKey,
            Exchange,
            S,
            S,
            MarketDataInstrumentKind,
            Kind,
        ),
    ) -> Self {
        let instrument = Keyed::new(instrument_id, (base, quote, instrument_kind).into());

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

impl<Exchange, Instrument, Kind> Validator for Subscription<Exchange, Instrument, Kind>
where
    Exchange: Connector,
    Instrument: InstrumentData,
{
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        // Validate the Exchange supports the Subscription InstrumentKind
        if exchange_supports_instrument_kind(Exchange::ID, self.instrument.kind()) {
            Ok(self)
        } else {
            Err(SocketError::Unsupported {
                entity: Exchange::ID.to_string(),
                item: self.instrument.kind().to_string(),
            })
        }
    }
}

/// Determines whether the [`Connector`] associated with this [`ExchangeId`] supports the
/// ingestion of market data for the provided [`MarketDataInstrumentKind`].
#[allow(clippy::match_like_matches_macro)]
pub fn exchange_supports_instrument_kind(
    exchange: ExchangeId,
    instrument_kind: &MarketDataInstrumentKind,
) -> bool {
    use barter_instrument::{
        exchange::ExchangeId::*, instrument::market_data::kind::MarketDataInstrumentKind::*,
    };

    match (exchange, instrument_kind) {
        // Spot
        (
            BinanceFuturesUsd | Bitmex | BybitPerpetualsUsd | GateioPerpetualsUsd
            | GateioPerpetualsBtc,
            Spot,
        ) => false,
        (_, Spot) => true,

        // Future
        (GateioFuturesUsd | GateioFuturesBtc | Okx, Future { .. }) => true,
        (_, Future { .. }) => false,

        // Perpetual
        (
            BinanceFuturesUsd | Bitmex | Okx | BybitPerpetualsUsd | GateioPerpetualsUsd
            | GateioPerpetualsBtc,
            Perpetual,
        ) => true,
        (_, Perpetual) => false,

        // Option
        (GateioOptions | Okx, Option { .. }) => true,
        (_, Option { .. }) => false,
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
        if exchange_supports_instrument_kind_sub_kind(
            &self.exchange,
            self.instrument.kind(),
            self.kind,
        ) {
            Ok(self)
        } else {
            Err(SocketError::Unsupported {
                entity: self.exchange.to_string(),
                item: format!("({}, {})", self.instrument.kind(), self.kind),
            })
        }
    }
}

/// Determines whether the [`Connector`] associated with this [`ExchangeId`] supports the
/// ingestion of market data for the provided [`MarketDataInstrumentKind`] and [`SubKind`] combination.
pub fn exchange_supports_instrument_kind_sub_kind(
    exchange_id: &ExchangeId,
    instrument_kind: &MarketDataInstrumentKind,
    sub_kind: SubKind,
) -> bool {
    use ExchangeId::*;
    use MarketDataInstrumentKind::*;
    use SubKind::*;

    match (exchange_id, instrument_kind, sub_kind) {
        (BinanceSpot, Spot, PublicTrades | OrderBooksL1 | OrderBooksL2) => true,
        (
            BinanceFuturesUsd,
            Perpetual,
            PublicTrades | OrderBooksL1 | OrderBooksL2 | Liquidations,
        ) => true,
        (Bitfinex, Spot, PublicTrades) => true,
        (Bitmex, Perpetual, PublicTrades) => true,
        (BybitSpot, Spot, PublicTrades | OrderBooksL1 | OrderBooksL2) => true,
        (BybitPerpetualsUsd, Perpetual, PublicTrades | OrderBooksL1 | OrderBooksL2) => true,
        (Coinbase, Spot, PublicTrades) => true,
        (GateioSpot, Spot, PublicTrades) => true,
        (GateioFuturesUsd, Future { .. }, PublicTrades) => true,
        (GateioFuturesBtc, Future { .. }, PublicTrades) => true,
        (GateioPerpetualsUsd, Perpetual, PublicTrades) => true,
        (GateioPerpetualsBtc, Perpetual, PublicTrades) => true,
        (GateioOptions, Option { .. }, PublicTrades) => true,
        (Kraken, Spot, PublicTrades | OrderBooksL1) => true,
        (Okx, Spot | Future { .. } | Perpetual | Option { .. }, PublicTrades) => true,

        (_, _, _) => false,
    }
}

/// Metadata generated from a collection of Barter [`Subscription`]s, including the exchange
/// specific subscription payloads that are sent to the exchange.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct SubscriptionMeta<InstrumentKey> {
    /// `HashMap` containing the mapping between a [`SubscriptionId`] and
    /// it's associated Barter [`MarketDataInstrument`].
    pub instrument_map: Map<InstrumentKey>,
    /// Collection of [`WsMessage`]s containing exchange specific subscription payloads to be sent.
    pub ws_subscriptions: Vec<WsMessage>,
}

/// New type`HashMap` that maps a [`SubscriptionId`] to some associated type `T`.
///
/// Used by [`ExchangeTransformer`](crate::transformer::ExchangeTransformer)s to identify the
/// Barter [`MarketDataInstrument`] associated with incoming exchange messages.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct Map<T>(pub FnvHashMap<SubscriptionId, T>);

impl<T> FromIterator<(SubscriptionId, T)> for Map<T> {
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = (SubscriptionId, T)>,
    {
        Self(iter.into_iter().collect::<FnvHashMap<SubscriptionId, T>>())
    }
}

impl<T> Map<T> {
    /// Find the `InstrumentKey` associated with the provided [`SubscriptionId`].
    pub fn find<SubId>(&self, id: &SubId) -> Result<&T, SocketError>
    where
        SubscriptionId: Borrow<SubId>,
        SubId: AsRef<str> + Hash + Eq + ?Sized,
    {
        self.0
            .get(id)
            .ok_or_else(|| SocketError::Unidentifiable(SubscriptionId(id.as_ref().to_smolstr())))
    }

    /// Find the mutable reference to `T` associated with the provided [`SubscriptionId`].
    pub fn find_mut<SubId>(&mut self, id: &SubId) -> Result<&mut T, SocketError>
    where
        SubscriptionId: Borrow<SubId>,
        SubId: AsRef<str> + Hash + Eq + ?Sized,
    {
        self.0
            .get_mut(id)
            .ok_or_else(|| SocketError::Unidentifiable(SubscriptionId(id.as_ref().to_smolstr())))
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
        use barter_instrument::instrument::market_data::MarketDataInstrument;

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
            use barter_instrument::instrument::market_data::MarketDataInstrument;

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

                serde_json::from_str::<Subscription<Okx, MarketDataInstrument, PublicTrades>>(
                    input,
                )
                .unwrap();
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

                serde_json::from_str::<Subscription<BinanceSpot, MarketDataInstrument, PublicTrades>>(input)
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

                serde_json::from_str::<
                    Subscription<BinanceFuturesUsd, MarketDataInstrument, OrderBooksL2>,
                >(input)
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

                serde_json::from_str::<
                    Subscription<GateioPerpetualsUsd, MarketDataInstrument, PublicTrades>,
                >(input)
                .unwrap();
            }
        }

        #[test]
        fn test_validate_bitfinex_public_trades() {
            struct TestCase {
                input: Subscription<Coinbase, MarketDataInstrument, PublicTrades>,
                expected:
                    Result<Subscription<Coinbase, MarketDataInstrument, PublicTrades>, SocketError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Valid Coinbase Spot PublicTrades subscription
                    input: Subscription::from((
                        Coinbase,
                        "base",
                        "quote",
                        MarketDataInstrumentKind::Spot,
                        PublicTrades,
                    )),
                    expected: Ok(Subscription::from((
                        Coinbase,
                        "base",
                        "quote",
                        MarketDataInstrumentKind::Spot,
                        PublicTrades,
                    ))),
                },
                TestCase {
                    // TC1: Invalid Coinbase FuturePerpetual PublicTrades subscription
                    input: Subscription::from((
                        Coinbase,
                        "base",
                        "quote",
                        MarketDataInstrumentKind::Perpetual,
                        PublicTrades,
                    )),
                    expected: Err(SocketError::Unsupported {
                        entity: "".to_string(),
                        item: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = test.input.validate();
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

        #[test]
        fn test_validate_okx_public_trades() {
            struct TestCase {
                input: Subscription<Okx, MarketDataInstrument, PublicTrades>,
                expected:
                    Result<Subscription<Okx, MarketDataInstrument, PublicTrades>, SocketError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Valid Okx Spot PublicTrades subscription
                    input: Subscription::from((
                        Okx,
                        "base",
                        "quote",
                        MarketDataInstrumentKind::Spot,
                        PublicTrades,
                    )),
                    expected: Ok(Subscription::from((
                        Okx,
                        "base",
                        "quote",
                        MarketDataInstrumentKind::Spot,
                        PublicTrades,
                    ))),
                },
                TestCase {
                    // TC1: Valid Okx FuturePerpetual PublicTrades subscription
                    input: Subscription::from((
                        Okx,
                        "base",
                        "quote",
                        MarketDataInstrumentKind::Perpetual,
                        PublicTrades,
                    )),
                    expected: Ok(Subscription::from((
                        Okx,
                        "base",
                        "quote",
                        MarketDataInstrumentKind::Perpetual,
                        PublicTrades,
                    ))),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = test.input.validate();
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

    mod instrument_map {
        use super::*;
        use barter_instrument::instrument::market_data::MarketDataInstrument;

        #[test]
        fn test_find_instrument() {
            // Initialise SubscriptionId-InstrumentKey HashMap
            let ids = Map(FnvHashMap::from_iter([(
                SubscriptionId::from("present"),
                MarketDataInstrument::from(("base", "quote", MarketDataInstrumentKind::Spot)),
            )]));

            struct TestCase {
                input: SubscriptionId,
                expected: Result<MarketDataInstrument, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: SubscriptionId (channel) is present in the HashMap
                    input: SubscriptionId::from("present"),
                    expected: Ok(MarketDataInstrument::from((
                        "base",
                        "quote",
                        MarketDataInstrumentKind::Spot,
                    ))),
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
                        panic!(
                            "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                        );
                    }
                }
            }
        }
    }
}

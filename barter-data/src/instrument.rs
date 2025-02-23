use barter_instrument::{
    Keyed,
    instrument::{
        Instrument,
        market_data::{MarketDataInstrument, kind::MarketDataInstrumentKind},
        name::InstrumentNameExchange,
    },
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Instrument related data that defines an associated unique `Id`.
///
/// Verbose `InstrumentData` is often used to subscribe to market data feeds, but it's unique `Id`
/// can then be used to key consumed [MarketEvents](crate::event::MarketEvent), significantly reducing
/// duplication in the case of complex instruments (eg/ options).
pub trait InstrumentData
where
    Self: Clone + Debug + Send + Sync,
{
    type Key: Debug + Clone + Eq + Send + Sync;
    fn key(&self) -> &Self::Key;
    fn kind(&self) -> &MarketDataInstrumentKind;
}

impl<InstrumentKey> InstrumentData for Keyed<InstrumentKey, MarketDataInstrument>
where
    InstrumentKey: Debug + Clone + Eq + Send + Sync,
{
    type Key = InstrumentKey;

    fn key(&self) -> &Self::Key {
        &self.key
    }

    fn kind(&self) -> &MarketDataInstrumentKind {
        &self.value.kind
    }
}

impl InstrumentData for MarketDataInstrument {
    type Key = Self;

    fn key(&self) -> &Self::Key {
        self
    }

    fn kind(&self) -> &MarketDataInstrumentKind {
        &self.kind
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct MarketInstrumentData<InstrumentKey> {
    pub key: InstrumentKey,
    pub name_exchange: InstrumentNameExchange,
    pub kind: MarketDataInstrumentKind,
}

impl<InstrumentKey> InstrumentData for MarketInstrumentData<InstrumentKey>
where
    InstrumentKey: Debug + Clone + Eq + Send + Sync,
{
    type Key = InstrumentKey;

    fn key(&self) -> &Self::Key {
        &self.key
    }

    fn kind(&self) -> &MarketDataInstrumentKind {
        &self.kind
    }
}

impl<InstrumentKey> std::fmt::Display for MarketInstrumentData<InstrumentKey>
where
    InstrumentKey: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}_{}_{}",
            self.key,
            self.name_exchange.as_ref(),
            self.kind
        )
    }
}

impl<ExchangeKey, AssetKey, InstrumentKey>
    From<&Keyed<InstrumentKey, Instrument<ExchangeKey, AssetKey>>>
    for MarketInstrumentData<InstrumentKey>
where
    InstrumentKey: Clone,
{
    fn from(value: &Keyed<InstrumentKey, Instrument<ExchangeKey, AssetKey>>) -> Self {
        Self {
            key: value.key.clone(),
            name_exchange: value.value.name_exchange.clone(),
            kind: MarketDataInstrumentKind::from(&value.value.kind),
        }
    }
}

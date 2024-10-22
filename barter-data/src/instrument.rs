use barter_instrument::{
    instrument::{kind::InstrumentKind, Instrument, InstrumentId},
    Keyed,
};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
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
    fn kind(&self) -> InstrumentKind;
}

impl<InstrumentKey> InstrumentData for Keyed<InstrumentKey, Instrument>
where
    InstrumentKey: Debug + Clone + Eq + Send + Sync,
{
    type Key = InstrumentKey;

    fn key(&self) -> &Self::Key {
        &self.key
    }

    fn kind(&self) -> InstrumentKind {
        self.value.kind
    }
}

impl InstrumentData for Instrument {
    type Key = Self;

    fn key(&self) -> &Self::Key {
        self
    }

    fn kind(&self) -> InstrumentKind {
        self.kind
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct MarketInstrumentData {
    pub id: InstrumentId,
    pub name_exchange: SmolStr,
    pub kind: InstrumentKind,
}

impl InstrumentData for MarketInstrumentData {
    type Key = InstrumentId;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn kind(&self) -> InstrumentKind {
        self.kind
    }
}

use barter_integration::model::instrument::{kind::InstrumentKind, Instrument};
use derive_more::{Constructor, Display};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Concise unique identifier for an instrument. Used to key
/// [MarketEvents](crate::event::MarketEvent) in a memory efficient way.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub struct InstrumentId(pub u64);

/// Instrument related data that defines an associated unique `Id`.
///
/// Verbose `InstrumentData` is often used to subscribe to market data feeds, but it's unique `Id`
/// can then be used to key consumed [MarketEvents](crate::event::MarketEvent), significantly reducing
/// duplication in the case of complex instruments (eg/ options).
pub trait InstrumentData
where
    Self: Clone + Debug + Send + Sync,
{
    type Key: Debug + Clone + Send + Sync;
    fn key(&self) -> &Self::Key;
    fn kind(&self) -> InstrumentKind;
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct KeyedInstrument<Id = InstrumentId> {
    pub id: Id,
    pub data: Instrument,
}

impl<Id> InstrumentData for KeyedInstrument<Id>
where
    Id: Debug + Clone + Send + Sync,
{
    type Key = Id;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn kind(&self) -> InstrumentKind {
        self.data.kind
    }
}

impl<Id> AsRef<Instrument> for KeyedInstrument<Id> {
    fn as_ref(&self) -> &Instrument {
        &self.data
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
    pub name_exchange: String,
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

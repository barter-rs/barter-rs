use crate::subscription::{SubKind, Subscription};
use barter_instrument::{
    exchange::ExchangeId,
    index::{error::IndexError, IndexedInstruments},
    instrument::{
        market_data::{kind::MarketDataInstrumentKind, MarketDataInstrument},
        InstrumentId, InstrumentIndex,
    },
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
pub struct MarketInstrumentData {
    pub id: InstrumentId,
    pub name_exchange: SmolStr,
    pub kind: MarketDataInstrumentKind,
}

impl InstrumentData for MarketInstrumentData {
    type Key = InstrumentId;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn kind(&self) -> &MarketDataInstrumentKind {
        &self.kind
    }
}

pub fn index_market_data_subscriptions<SubBatchIter, SubIter, Sub>(
    instruments: &IndexedInstruments,
    batches: SubBatchIter,
) -> Result<
    Vec<Vec<Subscription<ExchangeId, Keyed<InstrumentIndex, MarketDataInstrument>>>>,
    IndexError,
>
where
    SubBatchIter: IntoIterator<Item = SubIter>,
    SubIter: IntoIterator<Item = Sub>,
    Sub: Into<Subscription<ExchangeId, MarketDataInstrument, SubKind>>,
{
    batches
        .into_iter()
        .map(|batch| batch
            .into_iter()
            .map(|sub| {
                let sub = sub.into();

                let base_index = instruments.find_asset_index(sub.exchange, &sub.instrument.base)?;
                let quote_index = instruments.find_asset_index(sub.exchange, &sub.instrument.quote)?;

                let find_instrument = |exchange, kind, base, quote| {
                    instruments
                        .instruments()
                        .iter()
                        .find_map(|indexed| {
                            (
                                indexed.value.exchange.value == exchange
                                    && indexed.value.kind.eq_market_data_instrument_kind(kind)
                                    && indexed.value.underlying.base == base
                                    && indexed.value.underlying.quote == quote
                            ).then_some(indexed.key)
                        })
                        .ok_or(IndexError::InstrumentIndex(format!(
                            "Instrument: ({}, {}, {}, {}) must be present in indexed instruments: {:?}",
                            exchange, kind, base, quote, instruments.instruments()
                        )))
                };

                let instrument_index = find_instrument(sub.exchange, &sub.instrument.kind, base_index, quote_index)?;

                Ok(Subscription {
                    exchange: sub.exchange,
                    instrument: Keyed::new(instrument_index, sub.instrument),
                    kind: sub.kind,
                })
            })
            .collect()
        )
        .collect()
}

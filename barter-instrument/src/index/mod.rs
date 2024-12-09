use crate::{
    asset::{name::AssetNameInternal, Asset, AssetIndex, ExchangeAsset},
    exchange::{ExchangeId, ExchangeIndex},
    index::{builder::IndexedInstrumentsBuilder, error::IndexError},
    instrument::{name::InstrumentNameInternal, Instrument, InstrumentIndex},
    Keyed,
};
use serde::{Deserialize, Serialize};

pub mod builder;
pub mod error;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct IndexedInstruments {
    pub exchanges: Vec<Keyed<ExchangeIndex, ExchangeId>>,
    pub instruments:
        Vec<Keyed<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>>,
    pub assets: Vec<Keyed<AssetIndex, ExchangeAsset<Asset>>>,
}

impl IndexedInstruments {
    pub fn new<Iter>(instruments: Iter) -> Result<Self, IndexError>
    where
        Iter: IntoIterator<Item = Instrument<ExchangeId, Asset>>,
    {
        instruments
            .into_iter()
            .fold(Self::builder(), |mut builder, instrument| {
                builder.add_instrument(instrument);
                builder
            })
            .build()
    }

    pub fn builder() -> IndexedInstrumentsBuilder {
        IndexedInstrumentsBuilder::default()
    }

    pub fn find_exchange_index(&self, exchange: ExchangeId) -> Result<ExchangeIndex, IndexError> {
        find_exchange_by_exchange_id(&self.exchanges, &exchange)
    }

    pub fn find_asset_index(
        &self,
        exchange: ExchangeId,
        name: &AssetNameInternal,
    ) -> Result<AssetIndex, IndexError> {
        find_asset_by_exchange_and_name_internal(&self.assets, exchange, name)
    }

    pub fn find_instrument_index(
        &self,
        exchange: ExchangeId,
        name: &InstrumentNameInternal,
    ) -> Result<InstrumentIndex, IndexError> {
        self.instruments
            .iter()
            .find_map(|indexed| {
                (indexed.value.exchange.value == exchange && indexed.value.name_internal == *name)
                    .then_some(indexed.key)
            })
            .ok_or(IndexError::AssetIndex(format!(
                "Asset: ({}, {}) must be present in indexed instrument assets: {:?}",
                exchange, name, self.assets
            )))
    }
}

fn find_exchange_by_exchange_id(
    haystack: &[Keyed<ExchangeIndex, ExchangeId>],
    needle: &ExchangeId,
) -> Result<ExchangeIndex, IndexError> {
    haystack
        .iter()
        .find_map(|indexed| (indexed.value == *needle).then_some(indexed.key))
        .ok_or(IndexError::ExchangeIndex(format!(
            "Exchange: {} must be present in indexed instrument exchanges: {:?}",
            needle, haystack
        )))
}

fn find_asset_by_exchange_and_name_internal(
    haystack: &[Keyed<AssetIndex, ExchangeAsset<Asset>>],
    needle_exchange: ExchangeId,
    needle_name: &AssetNameInternal,
) -> Result<AssetIndex, IndexError> {
    haystack
        .iter()
        .find_map(|indexed| {
            (indexed.value.exchange == needle_exchange
                && indexed.value.asset.name_internal == *needle_name)
                .then_some(indexed.key)
        })
        .ok_or(IndexError::AssetIndex(format!(
            "Asset: ({}, {}) must be present in indexed instrument assets: {:?}",
            needle_exchange, needle_name, haystack
        )))
}

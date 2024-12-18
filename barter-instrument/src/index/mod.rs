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

/// Indexed collection of exchanges, assets, and instruments.
///
/// Initialise incrementally via the [`IndexedInstrumentsBuilder`], or all at once via the
/// constructor.
///
/// The indexed collection is useful for creating efficient O(1) constant lookup state management
/// systems where the state is keyed on an instrument, asset, or exchange.
///
/// For example uses cases, see the central `barter` crate `EngineState` design.
///
/// # Index Relationships
/// - `ExchangeIndex`: Unique index for each [`ExchangeId`] added during initialisation.
/// - `InstrumentIndex`: Unique identifier for each [`Instrument`] added during initialisation.
/// - `AssetIndex`: Unique identifier for each [`ExchangeAsset>`] added during initialisation.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct IndexedInstruments {
    exchanges: Vec<Keyed<ExchangeIndex, ExchangeId>>,
    assets: Vec<Keyed<AssetIndex, ExchangeAsset<Asset>>>,
    instruments:
        Vec<Keyed<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>>,
}

impl IndexedInstruments {
    /// Initialises a new `IndexedInstruments` from an iterator of [`Instrument`]s.
    ///
    /// This method indexes all unique exchanges, assets, and instruments, creating efficient
    /// lookup tables for each entity type.
    ///
    /// Note that once an `IndexedInstruments` has been constructed, it cannot be mutated (this
    /// could invalidate existing index lookup tables).
    ///
    /// For incremental initialisation, see the [`IndexedInstrumentsBuilder`].
    pub fn new<Iter>(instruments: Iter) -> Self
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

    /// Returns a new [`IndexedInstrumentsBuilder`] useful for incremental initialisation of
    /// `IndexedInstruments`.
    pub fn builder() -> IndexedInstrumentsBuilder {
        IndexedInstrumentsBuilder::default()
    }

    /// Returns a reference to the [`ExchangeIndex`] <--> [`ExchangeId`] associations.
    pub fn exchanges(&self) -> &[Keyed<ExchangeIndex, ExchangeId>] {
        &self.exchanges
    }

    /// Returns a reference to the [`AssetIndex`] <--> [`ExchangeAsset>`] associations.
    pub fn assets(&self) -> &[Keyed<AssetIndex, ExchangeAsset<Asset>>] {
        &self.assets
    }

    /// Returns a reference to the [`InstrumentIndex`] <--> [`Instrument`] associations.
    pub fn instruments(
        &self,
    ) -> &[Keyed<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>] {
        &self.instruments
    }

    /// Finds the [`ExchangeIndex`] associated with the provided [`ExchangeId`].
    ///
    /// # Arguments
    /// * `exchange` - The exchange ID to look up
    ///
    /// # Returns
    /// * `Ok(ExchangeIndex)` - exchange found.
    /// * `Err(IndexError)` - exchange not found.
    pub fn find_exchange_index(&self, exchange: ExchangeId) -> Result<ExchangeIndex, IndexError> {
        find_exchange_by_exchange_id(&self.exchanges, &exchange)
    }

    /// Finds the [`AssetIndex`] associated with the provided `ExchangeId` and `AssetNameInterval`.
    ///
    /// # Arguments
    /// * `exchange` - The `ExchangeId` associated with the asset.
    /// * `name` - The `AssetNameInternal` associated with the asset (eg/ "btc", "usdt", etc).
    ///
    /// # Returns
    /// * `Ok(AssetIndex)` - exchange asset found.
    /// * `Err(IndexError)` - exchange asset not found.
    pub fn find_asset_index(
        &self,
        exchange: ExchangeId,
        name: &AssetNameInternal,
    ) -> Result<AssetIndex, IndexError> {
        find_asset_by_exchange_and_name_internal(&self.assets, exchange, name)
    }

    /// Finds the [`InstrumentIndex`] associated with the provided `ExchangeId` and
    /// `InstrumentNameInternal`.
    ///
    /// # Arguments
    /// * `exchange` - The `ExchangeId` associated with the instrument.
    /// * `name` - The `InstrumentNameInternal` associated with the instrument (eg/ binance_spot_btc_usdt).
    ///
    /// # Returns
    /// * `Ok(AssetIndex)` - instrument found.
    /// * `Err(IndexError)` - instrument not found.
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

use crate::{FnvIndexMap, FnvIndexSet};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    exchange::ExchangeIndex,
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};

/// Indexed instrument map used by an execution manager. Associated internal representation of
/// instruments and assets with the exchange representation.
///
/// When an Engine [`ExecutionRequest`](super::ExecutionRequest) is received by the execution manager,
/// it needs to determine the exchange representation of the associated assets and instruments.
///
/// Similarly, when the execution manager received an [`AccountEvent`](super::AccountEvent)
/// from the exchange API, it needs to determine the internal representation of the associated
/// assets and instruments.
///
/// eg/ `InstrumentNameExchange("XBT-USDT")` <--> `InstrumentIndex(1)` <br>
/// eg/ `AssetNameExchange("XBT")` <--> `AssetIndex(1)`
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ExecutionInstrumentMap {
    pub exchange: ExchangeIndex,
    pub assets: FnvIndexSet<AssetNameExchange>,
    pub instruments: FnvIndexSet<InstrumentNameExchange>,
    pub asset_names: FnvHashMap<AssetNameExchange, AssetIndex>,
    pub instrument_names: FnvHashMap<InstrumentNameExchange, InstrumentIndex>,
}

impl ExecutionInstrumentMap {
    /// Construct a new [`Self`] using the provided indexed assets and instruments.
    pub fn new(
        exchange: ExchangeIndex,
        assets: FnvIndexMap<AssetIndex, AssetNameExchange>,
        instruments: FnvIndexMap<InstrumentIndex, InstrumentNameExchange>,
    ) -> Self {
        Self {
            exchange,
            asset_names: assets
                .iter()
                .map(|(key, value)| (value.clone(), *key))
                .collect(),
            instrument_names: instruments
                .iter()
                .map(|(key, value)| (value.clone(), *key))
                .collect(),
            assets: assets.into_values().collect(),
            instruments: instruments.into_values().collect(),
        }
    }

    pub fn exchange_assets(&self) -> impl Iterator<Item = &AssetNameExchange> {
        self.assets.iter()
    }

    pub fn exchange_instruments(&self) -> impl Iterator<Item = &InstrumentNameExchange> {
        self.instruments.iter()
    }

    pub fn find_asset_name_exchange(&self, asset: AssetIndex) -> &AssetNameExchange {
        self.assets
            .get_index(asset.index())
            .unwrap_or_else(|| panic!("ExecutionInstrumentMap does not contain: {asset}"))
    }

    pub fn find_asset_index(&self, asset: &AssetNameExchange) -> AssetIndex {
        self.asset_names
            .get(asset)
            .copied()
            .unwrap_or_else(|| panic!("ExecutionInstrumentMap does not contain: {asset}"))
    }

    pub fn find_instrument_name_exchange(
        &self,
        instrument: InstrumentIndex,
    ) -> &InstrumentNameExchange {
        self.instruments
            .get_index(instrument.index())
            .unwrap_or_else(|| panic!("ExecutionInstrumentMap does not contain: {instrument}"))
    }

    pub fn find_instrument_index(&self, instrument: &InstrumentNameExchange) -> InstrumentIndex {
        self.instrument_names
            .get(instrument)
            .copied()
            .unwrap_or_else(|| panic!("ExecutionInstrumentMap does not contain: {instrument}"))
    }
}

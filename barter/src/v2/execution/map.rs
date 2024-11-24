use crate::{
    v2::error::{IndexError, KeyError},
    FnvIndexMap, FnvIndexSet,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
    Keyed,
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
    pub exchange: Keyed<ExchangeIndex, ExchangeId>,
    pub assets: FnvIndexSet<AssetNameExchange>,
    pub instruments: FnvIndexSet<InstrumentNameExchange>,
    pub asset_names: FnvHashMap<AssetNameExchange, AssetIndex>,
    pub instrument_names: FnvHashMap<InstrumentNameExchange, InstrumentIndex>,
}

impl ExecutionInstrumentMap {
    /// Construct a new [`Self`] using the provided indexed assets and instruments.
    pub fn new(
        exchange: Keyed<ExchangeIndex, ExchangeId>,
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

    pub fn find_exchange_id(&self, exchange: ExchangeIndex) -> Result<ExchangeId, KeyError> {
        if self.exchange.key == exchange {
            Ok(self.exchange.value)
        } else {
            Err(KeyError::ExchangeId(format!(
                "ExecutionInstrumentMap does not contain {exchange}"
            )))
        }
    }

    pub fn find_exchange_index(&self, exchange: ExchangeId) -> Result<ExchangeIndex, IndexError> {
        if self.exchange.value == exchange {
            Ok(self.exchange.key)
        } else {
            Err(IndexError::ExchangeIndex(format!(
                "ExecutionInstrumentMap does not contain {exchange}"
            )))
        }
    }

    pub fn find_asset_name_exchange(
        &self,
        asset: AssetIndex,
    ) -> Result<&AssetNameExchange, KeyError> {
        self.assets.get_index(asset.index()).ok_or_else(|| {
            KeyError::AssetKey(format!("ExecutionInstrumentMap does not contain: {asset}"))
        })
    }

    pub fn find_asset_index(&self, asset: &AssetNameExchange) -> Result<AssetIndex, IndexError> {
        self.asset_names.get(asset).copied().ok_or_else(|| {
            IndexError::AssetIndex(format!("ExecutionInstrumentMap does not contain: {asset}"))
        })
    }

    pub fn find_instrument_name_exchange(
        &self,
        instrument: InstrumentIndex,
    ) -> Result<&InstrumentNameExchange, KeyError> {
        self.instruments
            .get_index(instrument.index())
            .ok_or_else(|| {
                KeyError::InstrumentKey(format!(
                    "ExecutionInstrumentMap does not contain: {instrument}"
                ))
            })
    }

    pub fn find_instrument_index(
        &self,
        instrument: &InstrumentNameExchange,
    ) -> Result<InstrumentIndex, IndexError> {
        self.instrument_names
            .get(instrument)
            .copied()
            .ok_or_else(|| {
                IndexError::InstrumentIndex(format!(
                    "ExecutionInstrumentMap does not contain: {instrument}"
                ))
            })
    }

    pub fn find_asset_name_exchange_unchecked(&self, asset: AssetIndex) -> &AssetNameExchange {
        self.find_asset_name_exchange(asset).unwrap()
    }

    pub fn find_asset_index_unchecked(&self, asset: &AssetNameExchange) -> AssetIndex {
        self.find_asset_index(asset).unwrap()
    }

    pub fn find_instrument_name_exchange_unchecked(
        &self,
        instrument: InstrumentIndex,
    ) -> &InstrumentNameExchange {
        self.find_instrument_name_exchange(instrument).unwrap()
    }

    pub fn find_instrument_index_unchecked(
        &self,
        instrument: &InstrumentNameExchange,
    ) -> InstrumentIndex {
        self.find_instrument_index(instrument).unwrap()
    }
}

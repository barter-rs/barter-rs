use crate::FnvIndexMap;
use barter_instrument::{asset::AssetIndex, instrument::InstrumentIndex, Keyed};
use fnv::FnvHashMap;
use smol_str::SmolStr;

pub type InstrumentNameExchange = SmolStr;
pub type AssetNameExchange = SmolStr;

#[derive(Debug)]
pub struct ExecutionInstrumentMap {
    pub assets: FnvIndexMap<AssetIndex, AssetNameExchange>,
    pub instruments: FnvIndexMap<InstrumentIndex, InstrumentNameExchange>,
    pub asset_names: FnvHashMap<AssetNameExchange, AssetIndex>,
    pub instrument_names: FnvHashMap<InstrumentNameExchange, InstrumentIndex>,
}

impl ExecutionInstrumentMap {
    pub fn new(
        assets: FnvIndexMap<AssetIndex, AssetNameExchange>,
        instruments: FnvIndexMap<InstrumentIndex, InstrumentNameExchange>
    ) -> Self
    {
        Self {
            asset_names: assets
                .iter()
                .map(|(key, value)| (
                    value.clone(), *key
                ))
                .collect(),
            instrument_names: instruments
                .iter()
                .map(|(key, value)| (
                    value.clone(), *key
                ))
                .collect(),
            assets,
            instruments,
        }
    }
}
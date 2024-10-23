use crate::FnvIndexMap;
use barter_instrument::{asset::AssetIndex, instrument::InstrumentIndex, Keyed};
use fnv::FnvHashMap;
use smol_str::SmolStr;

pub type InstrumentNameExchange = SmolStr;
pub type AssetNameExchange = SmolStr;

#[derive(Debug)]
pub struct ExecutionInstrumentMap<InstrumentData, AssetData> {
    pub instruments: FnvIndexMap<InstrumentIndex, Keyed<InstrumentNameExchange, InstrumentData>>,
    pub instrument_names: FnvHashMap<InstrumentNameExchange, InstrumentIndex>,
    pub assets: FnvIndexMap<AssetIndex, Keyed<AssetNameExchange, AssetData>>,
    pub asset_names: FnvHashMap<AssetNameExchange, AssetIndex>,
}

// #[derive(Debug, Clone, Default, Deserialize, Serialize, From)]
// pub struct Instruments<InstrumentKey: Eq + Hash, MarketState>(
//     pub FnvIndexMap<InstrumentKey, InstrumentState<InstrumentKey, MarketState>>,
// );

use fnv::{FnvHashMap};
use smol_str::SmolStr;
use barter_instrument::asset::AssetIndex;
use barter_instrument::instrument::InstrumentIndex;
use barter_instrument::Keyed;
use crate::FnvIndexMap;

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
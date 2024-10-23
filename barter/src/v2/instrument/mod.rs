use barter_instrument::{
    exchange::ExchangeId,
    instrument::{kind::InstrumentKind, spec::InstrumentSpec},
};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::fmt::Debug;

pub mod map;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Instrument<AssetKey> {
    pub exchange: ExchangeId,
    pub name_internal: SmolStr,
    pub name_exchange: SmolStr,
    pub kind: InstrumentKind,
    pub spec: InstrumentSpec<AssetKey>,
}

// #[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
// pub enum InstrumentKind<AssetKey = AssetId> {
//     Spot {
//         base_asset_id: AssetKey,
//     },
//     Perpetual {
//         base_asset_id: AssetKey,
//         quote_asset_id: AssetKey,
//         settlement_asset_id: AssetKey,
//     },
//     Future {
//         base_asset_id: AssetKey,
//         quote_asset_id: AssetKey,
//         settlement_asset_id: AssetKey,
//         contract: FutureContract,
//     },
//     Option {
//         base_asset_id: AssetKey,
//         quote_asset_id: AssetKey,
//         settlement_asset_id: AssetKey,
//         contract: OptionContract,
//     }
// }

use barter_instrument::{
    exchange::ExchangeId,
    instrument::{spec::InstrumentSpec},
};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::fmt::Debug;
use derive_more::Constructor;
use barter_instrument::instrument::kind::future::FutureContract;
use barter_instrument::instrument::kind::option::OptionContract;

pub mod map;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Instrument<AssetKey> {
    pub exchange: ExchangeId,
    pub name_internal: SmolStr,
    pub name_exchange: SmolStr,
    pub underlying: Underlying<AssetKey>,
    #[serde(alias = "instrument_kind")]
    pub kind: InstrumentKind<AssetKey>,
    pub spec: InstrumentSpec<AssetKey>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor)]
pub struct Underlying<AssetKey> {
    base: AssetKey,
    quote: AssetKey,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind<AssetKey> {
    Spot,
    Perpetual {
        settlement_asset: AssetKey,
    },
    Future {
        settlement_asset: AssetKey,
        contract: FutureContract,
    },
    Option {
        settlement_asset: AssetKey,
        contract: OptionContract,
    }
}

impl<AssetKey> InstrumentKind<AssetKey> {
    pub fn settlement_asset(&self) -> Option<&AssetKey> {
        match self {
            InstrumentKind::Spot => None,
            InstrumentKind::Perpetual { settlement_asset } => {
                Some(settlement_asset)
            },
            InstrumentKind::Future { settlement_asset, contract: _ } => {
                Some(settlement_asset)
            }
            InstrumentKind::Option { settlement_asset, contract: _ } => {
                Some(settlement_asset)
            }
        }
    }
}

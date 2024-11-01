use crate::instrument::{
    kind::{future::FutureContract, option::OptionContract},
    market_data::kind::MarketDataInstrumentKind,
};
use serde::{Deserialize, Serialize};

pub mod future;
pub mod option;

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
    },
}

impl<AssetKey> InstrumentKind<AssetKey> {
    pub fn settlement_asset(&self) -> Option<&AssetKey> {
        match self {
            InstrumentKind::Spot => None,
            InstrumentKind::Perpetual { settlement_asset } => Some(settlement_asset),
            InstrumentKind::Future {
                settlement_asset,
                contract: _,
            } => Some(settlement_asset),
            InstrumentKind::Option {
                settlement_asset,
                contract: _,
            } => Some(settlement_asset),
        }
    }

    pub fn eq_market_data_instrument_kind(&self, other: &MarketDataInstrumentKind) -> bool {
        match (self, other) {
            (Self::Spot, MarketDataInstrumentKind::Spot) => true,
            (Self::Perpetual { .. }, MarketDataInstrumentKind::Perpetual) => true,
            (Self::Future { contract, .. }, MarketDataInstrumentKind::Future(other_contract)) => {
                contract == other_contract
            }
            (Self::Option { contract, .. }, MarketDataInstrumentKind::Option(other_contract)) => {
                contract == other_contract
            }
            _ => false,
        }
    }
}

impl<AssetKey> From<InstrumentKind<AssetKey>> for MarketDataInstrumentKind {
    fn from(value: InstrumentKind<AssetKey>) -> Self {
        match value {
            InstrumentKind::Spot => MarketDataInstrumentKind::Spot,
            InstrumentKind::Perpetual { .. } => MarketDataInstrumentKind::Perpetual,
            InstrumentKind::Future { contract, .. } => MarketDataInstrumentKind::Future(contract),
            InstrumentKind::Option { contract, .. } => MarketDataInstrumentKind::Option(contract),
        }
    }
}

impl<AssetKey> From<&InstrumentKind<AssetKey>> for MarketDataInstrumentKind {
    fn from(value: &InstrumentKind<AssetKey>) -> Self {
        match value {
            InstrumentKind::Spot => MarketDataInstrumentKind::Spot,
            InstrumentKind::Perpetual { .. } => MarketDataInstrumentKind::Perpetual,
            InstrumentKind::Future { contract, .. } => MarketDataInstrumentKind::Future(*contract),
            InstrumentKind::Option { contract, .. } => {
                MarketDataInstrumentKind::Option(contract.clone())
            }
        }
    }
}

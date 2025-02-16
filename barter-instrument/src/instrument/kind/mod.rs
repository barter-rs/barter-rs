use crate::instrument::{
    kind::{future::FutureContract, option::OptionContract},
    market_data::kind::MarketDataInstrumentKind,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub mod future;
pub mod option;

/// [`Instrument`](super::Instrument) kind, one of `Spot`, `Perpetual`, `Future` and `Option`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind<AssetKey> {
    Spot,
    Perpetual {
        contract_size: Decimal,
        settlement_asset: AssetKey,
    },
    Future {
        contract: FutureContract,
        contract_size: Decimal,
        settlement_asset: AssetKey,
    },
    Option {
        contract: OptionContract,
        contract_size: Decimal,
        settlement_asset: AssetKey,
    },
}

impl<AssetKey> InstrumentKind<AssetKey> {
    /// Returns the `contract_size` value for the `InstrumentKind`.
    ///
    /// Note that `Spot` is always `Decimal::ONE`.
    pub fn contract_size(&self) -> Decimal {
        match self {
            InstrumentKind::Spot => Decimal::ONE,
            InstrumentKind::Perpetual { contract_size, .. } => *contract_size,
            InstrumentKind::Future { contract_size, .. } => *contract_size,
            InstrumentKind::Option { contract_size, .. } => *contract_size,
        }
    }

    /// For `Perpetual`, `Future` & `Option` variants of [`Self`], returns the settlement
    /// `AssetKey`, and `None` for Spot.
    pub fn settlement_asset(&self) -> Option<&AssetKey> {
        match self {
            InstrumentKind::Spot => None,
            InstrumentKind::Perpetual {
                settlement_asset, ..
            } => Some(settlement_asset),
            InstrumentKind::Future {
                settlement_asset, ..
            } => Some(settlement_asset),
            InstrumentKind::Option {
                settlement_asset, ..
            } => Some(settlement_asset),
        }
    }

    /// Determines if the provided [`MarketDataInstrumentKind`] is equivalent to [`Self`] (ignores
    /// settlement asset).
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

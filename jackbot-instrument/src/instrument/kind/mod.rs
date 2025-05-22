use crate::instrument::{
    kind::{future::FutureContract, option::OptionContract, perpetual::PerpetualContract},
    market_data::kind::{
        MarketDataFutureContract, MarketDataInstrumentKind, MarketDataOptionContract,
    },
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Defines an [`PerpetualContract`].
pub mod perpetual;

/// Defines an [`FutureContract`].
pub mod future;

/// Defines an [`OptionContract`].
pub mod option;

/// [`Instrument`](super::Instrument) kind, one of `Spot`, `Perpetual`, `Future` and `Option`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind<AssetKey> {
    Spot,
    Perpetual(PerpetualContract<AssetKey>),
    Future(FutureContract<AssetKey>),
    Option(OptionContract<AssetKey>),
}

impl<AssetKey> InstrumentKind<AssetKey> {
    /// Returns the `contract_size` value for the `InstrumentKind`.
    ///
    /// Note that `Spot` is always `Decimal::ONE`.
    pub fn contract_size(&self) -> Decimal {
        match self {
            InstrumentKind::Spot => Decimal::ONE,
            InstrumentKind::Perpetual(kind) => kind.contract_size,
            InstrumentKind::Future(kind) => kind.contract_size,
            InstrumentKind::Option(kind) => kind.contract_size,
        }
    }

    /// For `Perpetual`, `Future` & `Option` variants of [`Self`], returns the settlement
    /// `AssetKey`, and `None` for Spot.
    pub fn settlement_asset(&self) -> Option<&AssetKey> {
        match self {
            InstrumentKind::Spot => None,
            InstrumentKind::Perpetual(kind) => Some(&kind.settlement_asset),
            InstrumentKind::Future(kind) => Some(&kind.settlement_asset),
            InstrumentKind::Option(kind) => Some(&kind.settlement_asset),
        }
    }

    /// Determines if the provided [`MarketDataInstrumentKind`] is equivalent to [`Self`] (ignores
    /// settlement asset).
    pub fn eq_market_data_instrument_kind(&self, other: &MarketDataInstrumentKind) -> bool {
        match (self, other) {
            (Self::Spot, MarketDataInstrumentKind::Spot) => true,
            (Self::Perpetual(_), MarketDataInstrumentKind::Perpetual) => true,
            (Self::Future(contract), MarketDataInstrumentKind::Future(other_contract)) => {
                contract.expiry == other_contract.expiry
            }
            (Self::Option(contract), MarketDataInstrumentKind::Option(other_contract)) => {
                contract.kind == other_contract.kind
                    && contract.exercise == other_contract.exercise
                    && contract.expiry == other_contract.expiry
                    && contract.strike == other_contract.strike
            }
            _ => false,
        }
    }
}

impl<AssetKey> From<InstrumentKind<AssetKey>> for MarketDataInstrumentKind {
    fn from(value: InstrumentKind<AssetKey>) -> Self {
        match value {
            InstrumentKind::Spot => MarketDataInstrumentKind::Spot,
            InstrumentKind::Perpetual(_) => MarketDataInstrumentKind::Perpetual,
            InstrumentKind::Future(contract) => {
                MarketDataInstrumentKind::Future(MarketDataFutureContract {
                    expiry: contract.expiry,
                })
            }
            InstrumentKind::Option(contract) => {
                MarketDataInstrumentKind::Option(MarketDataOptionContract {
                    kind: contract.kind,
                    exercise: contract.exercise,
                    expiry: contract.expiry,
                    strike: contract.strike,
                })
            }
        }
    }
}

impl<AssetKey> From<&InstrumentKind<AssetKey>> for MarketDataInstrumentKind {
    fn from(value: &InstrumentKind<AssetKey>) -> Self {
        match value {
            InstrumentKind::Spot => MarketDataInstrumentKind::Spot,
            InstrumentKind::Perpetual(_) => MarketDataInstrumentKind::Perpetual,
            InstrumentKind::Future(contract) => {
                MarketDataInstrumentKind::Future(MarketDataFutureContract {
                    expiry: contract.expiry,
                })
            }
            InstrumentKind::Option(contract) => {
                MarketDataInstrumentKind::Option(MarketDataOptionContract {
                    kind: contract.kind,
                    exercise: contract.exercise,
                    expiry: contract.expiry,
                    strike: contract.strike,
                })
            }
        }
    }
}

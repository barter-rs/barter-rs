/// Configuration module for trading system components.
///
/// Provides data structures for configuring various aspects of a trading system,
/// including instruments and execution components.
use barter_execution::client::mock::MockExecutionConfig;
use barter_instrument::{
    Underlying,
    asset::{Asset, name::AssetNameExchange},
    exchange::ExchangeId,
    instrument::{
        Instrument,
        kind::{
            InstrumentKind, future::FutureContract, option::OptionContract,
            perpetual::PerpetualContract,
        },
        name::{InstrumentNameExchange, InstrumentNameInternal},
        quote::InstrumentQuoteAsset,
        spec::{InstrumentSpec, InstrumentSpecQuantity, OrderQuantityUnits},
    },
};
use derive_more::From;
use serde::{Deserialize, Serialize};

/// Top-level configuration for a full trading system.
///
/// Contains configuration for all instruments and execution components.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct SystemConfig {
    /// Configurations for all instruments the system will track.
    pub instruments: Vec<InstrumentConfig>,

    /// Configurations for all execution components.
    pub executions: Vec<ExecutionConfig>,
}

/// Convenient minimal instrument configuration, used to generate an [`Instrument`] on startup.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct InstrumentConfig {
    /// Exchange identifier where the instrument is traded.
    pub exchange: ExchangeId,

    /// Exchange-specific name for the instrument (e.g., "BTCUSDT").
    pub name_exchange: InstrumentNameExchange,

    /// Underlying asset pair for the instrument.
    pub underlying: Underlying<AssetNameExchange>,

    /// Quote asset for the instrument.
    pub quote: InstrumentQuoteAsset,

    /// Type of the instrument (spot, perpetual, future, option).
    pub kind: InstrumentKind<AssetNameExchange>,

    /// Optional additional specifications for the instrument.
    pub spec: Option<InstrumentSpec<AssetNameExchange>>,
}

/// Configuration for an execution link.
///
/// Represents different types of execution configurations,
/// currently only supporting mock execution for backtesting.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
#[serde(untagged)]
pub enum ExecutionConfig {
    /// Mock execution configuration for backtesting
    Mock(MockExecutionConfig),
}

impl From<InstrumentConfig> for Instrument<ExchangeId, Asset> {
    fn from(value: InstrumentConfig) -> Self {
        Self {
            exchange: value.exchange,
            name_internal: InstrumentNameInternal::new_from_exchange_underlying(
                value.exchange,
                &value.underlying.base,
                &value.underlying.quote,
            ),
            name_exchange: value.name_exchange,
            underlying: Underlying {
                base: Asset::new_from_exchange(value.underlying.base),
                quote: Asset::new_from_exchange(value.underlying.quote),
            },
            quote: value.quote,
            kind: match value.kind {
                InstrumentKind::Spot => InstrumentKind::Spot,
                InstrumentKind::Perpetual(contract) => {
                    InstrumentKind::Perpetual(PerpetualContract {
                        contract_size: contract.contract_size,
                        settlement_asset: Asset::new_from_exchange(contract.settlement_asset),
                    })
                }
                InstrumentKind::Future(contract) => InstrumentKind::Future(FutureContract {
                    contract_size: contract.contract_size,
                    settlement_asset: Asset::new_from_exchange(contract.settlement_asset),
                    expiry: contract.expiry,
                }),
                InstrumentKind::Option(contract) => InstrumentKind::Option(OptionContract {
                    contract_size: contract.contract_size,
                    settlement_asset: Asset::new_from_exchange(contract.settlement_asset),
                    kind: contract.kind,
                    exercise: contract.exercise,
                    expiry: contract.expiry,
                    strike: contract.strike,
                }),
            },
            spec: value.spec.map(|spec| InstrumentSpec {
                price: spec.price,
                quantity: InstrumentSpecQuantity {
                    unit: match spec.quantity.unit {
                        OrderQuantityUnits::Asset(asset) => {
                            OrderQuantityUnits::Asset(Asset::new_from_exchange(asset))
                        }
                        OrderQuantityUnits::Contract => OrderQuantityUnits::Contract,
                        OrderQuantityUnits::Quote => OrderQuantityUnits::Quote,
                    },
                    min: spec.quantity.min,
                    increment: spec.quantity.increment,
                },
                notional: spec.notional,
            }),
        }
    }
}

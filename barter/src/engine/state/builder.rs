use crate::engine::state::{
    EngineState, asset::generate_empty_indexed_asset_states,
    connectivity::generate_empty_indexed_connectivity_states,
    instrument::generate_empty_indexed_instrument_states, trading::TradingState,
};
use barter_execution::balance::{AssetBalance, Balance};
use barter_instrument::{
    Keyed,
    asset::{ExchangeAsset, name::AssetNameInternal},
    index::IndexedInstruments,
};
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, Utc};
use fnv::FnvHashMap;
use std::marker::PhantomData;
use tracing::debug;

/// Builder utility for an [`EngineState`] instance.
#[derive(Debug, Clone)]
pub struct EngineStateBuilder<'a, InstrumentData, Strategy, Risk> {
    pub instruments: &'a IndexedInstruments,
    pub strategy: Option<Strategy>,
    pub risk: Option<Risk>,
    pub trading_state: Option<TradingState>,
    pub time_engine_start: Option<DateTime<Utc>>,
    pub balances: FnvHashMap<ExchangeAsset<AssetNameInternal>, Balance>,
    phantom: PhantomData<InstrumentData>,
}

impl<'a, InstrumentData, Strategy, Risk> EngineStateBuilder<'a, InstrumentData, Strategy, Risk>
where
    InstrumentData: Default,
    Strategy: Default,
    Risk: Default,
{
    /// Construct a new `EngineStateBuilder` with a layout derived from [`IndexedInstruments`].
    ///
    /// Note that the rest of the [`EngineState`] data can be generated from defaults.
    ///
    /// Note that `ConnectivityStates` will be generated with
    /// [`generate_empty_indexed_connectivity_states`], defaulting to `Health::Reconnecting`.
    pub fn new(instruments: &'a IndexedInstruments) -> Self {
        Self {
            instruments,
            time_engine_start: None,
            trading_state: None,
            strategy: None,
            risk: None,
            balances: FnvHashMap::default(),
            phantom: PhantomData,
        }
    }

    /// Optionally provide the initial `TradingState`.
    ///
    /// Defaults to `TradingState::Disabled`.
    pub fn trading_state(self, value: TradingState) -> Self {
        Self {
            trading_state: Some(value),
            ..self
        }
    }

    /// Optionally provide the `time_engine_start`.
    ///
    /// Providing this is useful for back-test scenarios where the time should be seeded with a
    /// "historical" clock time (eg/ from first historical `MarketEvent`).
    ///
    /// Defaults to `Utc::now`
    pub fn time_engine_start(self, value: DateTime<Utc>) -> Self {
        Self {
            time_engine_start: Some(value),
            ..self
        }
    }

    /// Optionally provide the initial `StrategyState`.
    ///
    /// Defaults to `StrategyState::default()`.
    pub fn strategy(self, value: Strategy) -> Self {
        Self {
            strategy: Some(value),
            ..self
        }
    }

    /// Optionally provide the initial `RiskState`.
    ///
    /// Defaults to `RiskState::default()`.
    pub fn risk(self, value: Risk) -> Self {
        Self {
            risk: Some(value),
            ..self
        }
    }

    /// Optionally provide initial exchange asset `Balance`s.
    ///
    /// Useful for back-test scenarios where seeding EngineState with initial `Balance`s is
    /// required.
    ///
    /// Note the internal implementation uses a `HashMap`, so duplicate
    /// `ExchangeAsset<AssetNameInternal>` keys are overwritten.
    pub fn balances<BalanceIter, KeyedBalance>(mut self, balances: BalanceIter) -> Self
    where
        BalanceIter: IntoIterator<Item = KeyedBalance>,
        KeyedBalance: Into<Keyed<ExchangeAsset<AssetNameInternal>, Balance>>,
    {
        self.balances.extend(balances.into_iter().map(|keyed| {
            let Keyed { key, value } = keyed.into();

            (key, value)
        }));
        self
    }

    /// Use the builder data to generate the associated [`EngineState`].
    ///
    /// If optional data is not provided (eg/ Balances), default values are used (eg/ zero Balance).
    pub fn build(self) -> EngineState<InstrumentData, Strategy, Risk> {
        let Self {
            instruments,
            strategy,
            risk,
            time_engine_start,
            trading_state,
            balances,
            phantom: _,
        } = self;

        // Default to Utc::now if time_engine_start not provided
        let time_engine_start = time_engine_start.unwrap_or_else(|| {
            debug!("EngineStateBuilder using Utc::now as time_engine_start default");
            Utc::now()
        });

        // Update empty AssetStates from provided exchange asset Balances
        let mut assets = generate_empty_indexed_asset_states(instruments);
        for (key, balance) in balances {
            assets
                .asset_mut(&key)
                .update_from_balance(Snapshot(&AssetBalance {
                    asset: key.asset,
                    balance,
                    time_exchange: time_engine_start,
                }))
        }

        EngineState {
            trading: trading_state.unwrap_or_default(),
            connectivity: generate_empty_indexed_connectivity_states(instruments),
            assets,
            instruments: generate_empty_indexed_instrument_states::<InstrumentData>(
                instruments,
                time_engine_start,
            ),
            strategy: strategy.unwrap_or_default(),
            risk: risk.unwrap_or_default(),
        }
    }
}

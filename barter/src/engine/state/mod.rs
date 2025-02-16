use crate::engine::{
    state::{
        asset::{filter::AssetFilter, generate_empty_indexed_asset_states, AssetStates},
        builder::EngineStateBuilder,
        connectivity::{generate_empty_indexed_connectivity_states, ConnectivityStates},
        instrument::{
            filter::InstrumentFilter, generate_empty_indexed_instrument_states,
            generate_unindexed_instrument_account_snapshot, market_data::MarketDataState,
            InstrumentStates,
        },
        position::PositionExited,
        trading::TradingState,
    },
    Processor,
};
use barter_data::event::MarketEvent;
use barter_execution::{
    balance::AssetBalance, AccountEvent, AccountEventKind, UnindexedAccountSnapshot,
};
use barter_instrument::{
    asset::{AssetIndex, QuoteAsset},
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::InstrumentIndex,
};
use barter_integration::{collection::one_or_many::OneOrMany, snapshot::Snapshot};
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Asset-centric state and associated state management logic.
pub mod asset;

/// Connectivity state that tracks global connection health as well as the status of market data
/// and account connections for each exchange.
pub mod connectivity;

/// Instrument-centric state and associated state management logic.
pub mod instrument;

/// Defines a synchronous `OrderManager` that tracks the lifecycle of exchange orders.
pub mod order;

/// Position data structures and their associated state management logic.
pub mod position;

/// Defines the `TradingState` of the `Engine` (ie/ trading enabled & trading disabled), and it's
/// update logic.
pub mod trading;

/// [`EngineState`] builder utility.
pub mod builder;

/// Algorithmic trading `Engine` state.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct EngineState<Market, Strategy, Risk> {
    /// Current `TradingState` of the `Engine`.
    pub trading: TradingState,

    /// Global connection [`Health`](connectivity::Health), and health of the market data and
    /// account connections for each exchange.
    pub connectivity: ConnectivityStates,

    /// State of every asset (eg/ "btc", "usdt", etc.) being tracked by the `Engine`.
    pub assets: AssetStates,

    /// State of every instrument (eg/ "okx_spot_btc_usdt", "bybit_perpetual_btc_usdt", etc.)
    /// being tracked by the `Engine`.
    pub instruments: InstrumentStates<Market, ExchangeIndex, AssetIndex, InstrumentIndex>,

    /// Configurable global `Strategy` state.
    pub strategy: Strategy,

    /// Configurable global `RiskManager` state.
    pub risk: Risk,
}

impl<Market, Strategy, Risk> EngineState<Market, Strategy, Risk> {
    /// Construct an [`EngineStateBuilder`] to assist with `EngineState` initialisation.
    pub fn builder(
        instruments: &IndexedInstruments,
    ) -> EngineStateBuilder<'_, Market, Strategy, Risk>
    where
        Market: Default,
        Strategy: Default,
        Risk: Default,
    {
        EngineStateBuilder::new(instruments)
    }

    /// Updates the internal state from an `AccountEvent`.
    ///
    /// If the `AccountEvent` results in a new [`PositionExited`], that is returned.
    ///
    /// This method:
    /// - Sets the account [`ConnectivityState`](connectivity::ConnectivityState) to
    ///   [`Health::Healthy`](connectivity::Health::Healthy) if it was not previously.
    /// - Updates the `AssetState` and `InstrumentStates`.
    /// - Processes the `AccountEvent` with the configured `Strategy` and `RiskManager`
    ///   implementations.
    pub fn update_from_account(
        &mut self,
        event: &AccountEvent,
    ) -> Option<PositionExited<QuoteAsset>>
    where
        Strategy: for<'a> Processor<&'a AccountEvent>,
        Risk: for<'a> Processor<&'a AccountEvent>,
    {
        // Set exchange account connectivity to Healthy if it was Reconnecting
        self.connectivity.update_from_account_event(&event.exchange);

        let output = match &event.kind {
            AccountEventKind::Snapshot(snapshot) => {
                for balance in &snapshot.balances {
                    self.assets
                        .asset_index_mut(&balance.asset)
                        .update_from_balance(Snapshot(balance))
                }
                for instrument in &snapshot.instruments {
                    self.instruments
                        .instrument_index_mut(&instrument.instrument)
                        .update_from_account_snapshot(instrument)
                }
                None
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.assets
                    .asset_index_mut(&balance.0.asset)
                    .update_from_balance(balance.as_ref());
                None
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instruments
                    .instrument_index_mut(&order.0.key.instrument)
                    .update_from_order_snapshot(order.as_ref());
                None
            }
            AccountEventKind::OrderCancelled(response) => {
                self.instruments
                    .instrument_index_mut(&response.key.instrument)
                    .update_from_cancel_response(response);

                None
            }
            AccountEventKind::Trade(trade) => self
                .instruments
                .instrument_index_mut(&trade.instrument)
                .update_from_trade(trade),
        };

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);

        output
    }

    /// Updates the internal state from a `MarketEvent`.
    ///
    /// This method:
    /// - Sets the market data [`ConnectivityState`](connectivity::ConnectivityState) to
    ///   [`Health::Healthy`](connectivity::Health::Healthy) if it was not previously.
    /// - Updates the [`MarketDataState`] associated with the `MarketEvent` instrument.
    /// - Processes the `MarketEvent` with the configured `Strategy` and `RiskManager`
    ///   implementations.
    pub fn update_from_market(&mut self, event: &MarketEvent<InstrumentIndex, Market::EventKind>)
    where
        Market: MarketDataState,
        Strategy: for<'a> Processor<&'a MarketEvent<InstrumentIndex, Market::EventKind>>,
        Risk: for<'a> Processor<&'a MarketEvent<InstrumentIndex, Market::EventKind>>,
    {
        // Set exchange market data connectivity to Healthy if it was Reconnecting
        self.connectivity.update_from_market_event(&event.exchange);

        let instrument_state = self.instruments.instrument_index_mut(&event.instrument);

        instrument_state.market.process(event);
        self.strategy.process(event);
        self.risk.process(event);
    }
}

impl<Market, Strategy, Risk> From<&EngineState<Market, Strategy, Risk>>
    for FnvHashMap<ExchangeId, UnindexedAccountSnapshot>
{
    fn from(value: &EngineState<Market, Strategy, Risk>) -> Self {
        let EngineState {
            trading: _,
            connectivity,
            assets,
            instruments,
            strategy: _,
            risk: _,
        } = value;

        // Allocate appropriately
        let mut snapshots =
            FnvHashMap::with_capacity_and_hasher(connectivity.exchanges.len(), Default::default());

        // Insert UnindexedAccountSnapshot for each exchange
        for (index, exchange) in connectivity.exchange_ids().enumerate() {
            snapshots.insert(
                *exchange,
                UnindexedAccountSnapshot {
                    exchange: *exchange,
                    balances: assets
                        .filtered(&AssetFilter::Exchanges(OneOrMany::One(*exchange)))
                        .map(AssetBalance::from)
                        .collect(),
                    instruments: instruments
                        .instruments(&InstrumentFilter::Exchanges(OneOrMany::One(ExchangeIndex(
                            index,
                        ))))
                        .map(|snapshot| {
                            generate_unindexed_instrument_account_snapshot(*exchange, snapshot)
                        })
                        .collect::<Vec<_>>(),
                },
            );
        }

        snapshots
    }
}

/// Generates an indexed [`EngineState`] containing the provided `TradingState`, `Strategy` state,
/// and `Risk` state. All other data is set to default values.
pub fn generate_empty_indexed_engine_state<Market, Strategy, Risk>(
    trading_state: TradingState,
    instruments: &IndexedInstruments,
    time_engine_start: DateTime<Utc>,
    strategy: Strategy,
    risk: Risk,
) -> EngineState<Market, Strategy, Risk>
where
    Market: Default,
{
    EngineState {
        trading: trading_state,
        connectivity: generate_empty_indexed_connectivity_states(instruments),
        assets: generate_empty_indexed_asset_states(instruments),
        instruments: generate_empty_indexed_instrument_states::<Market>(
            instruments,
            time_engine_start,
        ),
        strategy,
        risk,
    }
}

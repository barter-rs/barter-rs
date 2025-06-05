use crate::engine::{
    Processor,
    state::{
        asset::{AssetStates, filter::AssetFilter},
        builder::EngineStateBuilder,
        connectivity::ConnectivityStates,
        instrument::{
            InstrumentStates, data::InstrumentDataState, filter::InstrumentFilter,
            generate_unindexed_instrument_account_snapshot,
        },
        position::PositionExited,
        trading::TradingState,
    },
};
use barter_data::event::MarketEvent;
use barter_execution::{
    AccountEvent, AccountEventKind, UnindexedAccountSnapshot, balance::AssetBalance,
};
use barter_instrument::{
    Keyed,
    asset::{AssetIndex, QuoteAsset},
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::{Instrument, InstrumentIndex},
};
use barter_integration::{collection::one_or_many::OneOrMany, snapshot::Snapshot};
use derive_more::Constructor;
use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Asset-centric state and associated state management logic.
pub mod asset;

/// Connectivity state that tracks global connection health as well as the status of market data
/// and account connections for each exchange.
pub mod connectivity;

/// Instrument-level state and associated state management logic.
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

/// Defines a default `GlobalData` implementation that can be used for systems which require no
/// specific global data.
pub mod global;

/// Algorithmic trading `Engine` state.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct EngineState<GlobalData, InstrumentData> {
    /// Current `TradingState` of the `Engine`.
    pub trading: TradingState,

    /// Configurable `GlobalData` state.
    pub global: GlobalData,

    /// Global connection [`Health`](connectivity::Health), and health of the market data and
    /// account connections for each exchange.
    pub connectivity: ConnectivityStates,

    /// State of every asset (eg/ "btc", "usdt", etc.) being tracked by the `Engine`.
    pub assets: AssetStates,

    /// State of every instrument (eg/ "okx_spot_btc_usdt", "bybit_perpetual_btc_usdt", etc.)
    /// being tracked by the `Engine`.
    pub instruments: InstrumentStates<InstrumentData, ExchangeIndex, AssetIndex, InstrumentIndex>,
}

impl<GlobalData, InstrumentData> EngineState<GlobalData, InstrumentData> {
    /// Construct an [`EngineStateBuilder`] to assist with `EngineState` initialisation.
    pub fn builder<FnInstrumentData>(
        instruments: &IndexedInstruments,
        global: GlobalData,
        instrument_data_init: FnInstrumentData,
    ) -> EngineStateBuilder<'_, GlobalData, FnInstrumentData>
    where
        FnInstrumentData: Fn(
            &Keyed<InstrumentIndex, Instrument<Keyed<ExchangeIndex, ExchangeId>, AssetIndex>>,
        ) -> InstrumentData,
    {
        EngineStateBuilder::new(instruments, global, instrument_data_init)
    }

    /// Updates the internal state from an `AccountEvent`.
    ///
    /// If the `AccountEvent` results in a new [`PositionExited`], that is returned.
    ///
    /// This method:
    /// - Sets the account [`ConnectivityState`](connectivity::ConnectivityState) to
    ///   [`Health::Healthy`](connectivity::Health::Healthy) if it was not previously.
    /// - Updates the `GlobalData` with the `AccountEvent`.
    /// - Updates the associated `AssetStates` and `InstrumentStates` with the `AccountEvent`.
    pub fn update_from_account(
        &mut self,
        event: &AccountEvent,
    ) -> Option<PositionExited<QuoteAsset>>
    where
        GlobalData: for<'a> Processor<&'a AccountEvent>,
        InstrumentData: for<'a> Processor<&'a AccountEvent>,
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
                    let instrument_state = self
                        .instruments
                        .instrument_index_mut(&instrument.instrument);

                    instrument_state.update_from_account_snapshot(instrument);
                    instrument_state.data.process(event);
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
                let instrument_state = self
                    .instruments
                    .instrument_index_mut(&order.value().key.instrument);

                instrument_state.update_from_order_snapshot(order.as_ref());
                instrument_state.data.process(event);
                None
            }
            AccountEventKind::OrderCancelled(response) => {
                let instrument_state = self
                    .instruments
                    .instrument_index_mut(&response.key.instrument);

                instrument_state.update_from_cancel_response(response);
                instrument_state.data.process(event);
                None
            }
            AccountEventKind::Trade(trade) => {
                let instrument_state = self.instruments.instrument_index_mut(&trade.instrument);

                instrument_state.data.process(event);
                instrument_state.update_from_trade(trade)
            }
        };

        // Update any user provided GlobalData State
        self.global.process(event);

        output
    }

    /// Updates the internal state from a `MarketEvent`.
    ///
    /// This method:
    /// - Sets the market data [`ConnectivityState`](connectivity::ConnectivityState) to
    ///   [`Health::Healthy`](connectivity::Health::Healthy) if it was not previously.
    /// - Updates the `GlobalData` with the `MarketEvent`.
    /// - Updates the associated [`InstrumentDataState`] with the `MarketEvent`.
    pub fn update_from_market(
        &mut self,
        event: &MarketEvent<InstrumentIndex, InstrumentData::MarketEventKind>,
    ) where
        GlobalData:
            for<'a> Processor<&'a MarketEvent<InstrumentIndex, InstrumentData::MarketEventKind>>,
        InstrumentData: InstrumentDataState,
    {
        // Set exchange market data connectivity to Healthy if it was Reconnecting
        self.connectivity.update_from_market_event(&event.exchange);

        let instrument_state = self.instruments.instrument_index_mut(&event.instrument);

        self.global.process(event);
        instrument_state.data.process(event);
    }
}

impl<GlobalData, InstrumentData> From<&EngineState<GlobalData, InstrumentData>>
    for FnvHashMap<ExchangeId, UnindexedAccountSnapshot>
{
    fn from(value: &EngineState<GlobalData, InstrumentData>) -> Self {
        let EngineState {
            trading: _,
            global: _,
            connectivity,
            assets,
            instruments,
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

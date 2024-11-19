use crate::v2::{
    engine::{
        state::{
            asset::{manager::AssetStateManager, AssetState, AssetStates},
            connectivity::{manager::ConnectivityManager, ConnectivityState, ConnectivityStates},
            instrument::{
                manager::InstrumentStateManager, market_data::MarketDataState, InstrumentStates,
            },
            order::{in_flight_recorder::InFlightRequestRecorder, manager::OrderManager},
            trading::{manager::TradingStateManager, TradingState},
        },
        Processor,
    },
    execution::{AccountEvent, AccountEventKind},
    order::{Order, RequestCancel, RequestOpen},
    Snapshot,
};
use barter_data::event::MarketEvent;
use barter_instrument::{
    asset::{name::AssetNameInternal, AssetIndex, ExchangeAsset},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod asset;
pub mod connectivity;
pub mod instrument;
pub mod order;
pub mod trading;

// Todo:
//  - Maybe introduce State machine for dealing with connectivity VecMap issue...
//    '--> could only check if a new Account/Market event updates to Connected if we are in
//         State=Unhealthy, that way we are only doing expensive lookup in that case

pub type IndexedEngineState<Market, Strategy, Risk> =
    EngineState<Market, Strategy, Risk, ExchangeIndex, AssetIndex, InstrumentIndex>;

// Todo: Move other Manager impls to where they are defined
pub trait StateManager<ExchangeKey, AssetKey, InstrumentKey>
where
    Self: TradingStateManager
        + ConnectivityManager<ExchangeId>
        + AssetStateManager<AssetKey>
        + InstrumentStateManager<InstrumentKey, ExchangeKey = ExchangeKey, AssetKey = AssetKey>,
{
    type MarketState;
    type MarketEventKind;

    fn update_from_account(&mut self, event: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>);
    fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey, Self::MarketEventKind>);
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey> {
    pub trading: TradingState,
    pub connectivity: ConnectivityStates,
    pub assets: AssetStates,
    pub instruments: InstrumentStates<Market, ExchangeKey, AssetKey, InstrumentKey>,
    pub strategy: Strategy,
    pub risk: Risk,
}

impl<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
    StateManager<ExchangeKey, AssetKey, InstrumentKey>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
where
    Self: TradingStateManager
        + ConnectivityManager<ExchangeId>
        + AssetStateManager<AssetKey>
        + InstrumentStateManager<
            InstrumentKey,
            ExchangeKey = ExchangeKey,
            AssetKey = AssetKey,
            Market = Market,
        >,
    Market: MarketDataState<InstrumentKey>,
    Strategy: for<'a> Processor<&'a AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey, Market::EventKind>>,
    Risk: for<'a> Processor<&'a AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey, Market::EventKind>>,
    ExchangeKey: Debug + Clone,
    AssetKey: Debug,
    InstrumentKey: Debug + Clone,
{
    type MarketState = Market;
    type MarketEventKind = Market::EventKind;

    fn update_from_account(&mut self, event: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) {
        // Todo: set exchange ConnectivityState to healthy if unhealthy

        match &event.kind {
            AccountEventKind::Snapshot(snapshot) => {
                for balance in &snapshot.balances {
                    self.asset_mut(&balance.asset)
                        .update_from_balance(Snapshot(balance))
                }
                for instrument in &snapshot.instruments {
                    self.instrument_mut(&instrument.position.instrument)
                        .update_from_account_snapshot(instrument)
                }
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.asset_mut(&balance.0.asset)
                    .update_from_balance(balance.as_ref());
            }
            AccountEventKind::PositionSnapshot(position) => {
                self.instrument_mut(&position.0.instrument)
                    .update_from_position_snapshot(position.as_ref());
            }
            AccountEventKind::OrderSnapshot(order) => self
                .instrument_mut(&order.0.instrument)
                .orders
                .update_from_order_snapshot(order.as_ref()),
            AccountEventKind::OrderOpened(response) => self
                .instrument_mut(&response.instrument)
                .orders
                .update_from_open(response),
            AccountEventKind::OrderCancelled(response) => self
                .instrument_mut(&response.instrument)
                .orders
                .update_from_cancel(response),
            AccountEventKind::Trade(trade) => {
                self.instrument_mut(&trade.instrument)
                    .update_from_trade(trade);
            }
        }

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);
    }

    fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey, Self::MarketEventKind>) {
        // Todo: set exchange ConnectivityState to healthy if unhealthy

        // Todo: uncomment once it works
        self.instrument_mut(&event.instrument).market.process(event);
        self.strategy.process(event);
        self.risk.process(event);
    }
}

impl<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
    InFlightRequestRecorder<ExchangeKey, InstrumentKey>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
where
    Self: InstrumentStateManager<InstrumentKey, ExchangeKey = ExchangeKey>,
    ExchangeKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    fn record_in_flight_cancel(
        &mut self,
        request: &Order<ExchangeKey, InstrumentKey, RequestCancel>,
    ) {
        self.instrument_mut(&request.instrument)
            .orders
            .record_in_flight_cancel(request);
    }

    fn record_in_flight_open(&mut self, request: &Order<ExchangeKey, InstrumentKey, RequestOpen>) {
        self.instrument_mut(&request.instrument)
            .orders
            .record_in_flight_open(request);
    }
}

impl<Market, Strategy, Risk, AssetKey, InstrumentKey> ConnectivityManager<ExchangeIndex>
    for EngineState<Market, Strategy, Risk, ExchangeIndex, AssetKey, InstrumentKey>
{
    fn connectivity(&self, key: &ExchangeIndex) -> &ConnectivityState {
        self.connectivity
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn connectivity_mut(&mut self, key: &ExchangeIndex) -> &mut ConnectivityState {
        self.connectivity
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey> ConnectivityManager<ExchangeId>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
{
    fn connectivity(&self, key: &ExchangeId) -> &ConnectivityState {
        self.connectivity
            .0
            .get(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn connectivity_mut(&mut self, key: &ExchangeId) -> &mut ConnectivityState {
        self.connectivity
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk, ExchangeKey, InstrumentKey> AssetStateManager<AssetIndex>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetIndex, InstrumentKey>
{
    fn asset(&self, key: &AssetIndex) -> &AssetState {
        self.assets
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }

    fn asset_mut(&mut self, key: &AssetIndex) -> &mut AssetState {
        self.assets
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk, ExchangeKey, InstrumentKey>
    AssetStateManager<ExchangeAsset<AssetNameInternal>>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetNameInternal, InstrumentKey>
{
    fn asset(&self, key: &ExchangeAsset<AssetNameInternal>) -> &AssetState {
        self.assets
            .0
            .get(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }

    fn asset_mut(&mut self, key: &ExchangeAsset<AssetNameInternal>) -> &mut AssetState {
        self.assets
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }
}

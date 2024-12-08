use crate::engine::{
    state::{
        asset::{manager::AssetStateManager, AssetStates},
        connectivity::{manager::ConnectivityManager, ConnectivityStates},
        instrument::{
            manager::InstrumentStateManager, market_data::MarketDataState, InstrumentStates,
        },
        order::manager::OrderManager,
        trading::{manager::TradingStateManager, TradingState},
    },
    Processor,
};
use barter_data::event::MarketEvent;
use barter_execution::{AccountEvent, AccountEventKind};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
};
use barter_integration::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub mod asset;
pub mod connectivity;
pub mod instrument;
pub mod order;
pub mod position;
pub mod trading;

pub type IndexedEngineState<Market, Strategy, Risk> =
    EngineState<Market, Strategy, Risk, ExchangeIndex, AssetIndex, InstrumentIndex>;

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
    AssetKey: Debug + Clone + PartialEq,
    InstrumentKey: Debug + Clone + PartialEq,
{
    type MarketState = Market;
    type MarketEventKind = Market::EventKind;

    fn update_from_account(&mut self, event: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>) {
        // Todo: set execution ConnectivityState to healthy if unhealthy

        match &event.kind {
            AccountEventKind::Snapshot(snapshot) => {
                for balance in &snapshot.balances {
                    self.asset_mut(&balance.asset)
                        .update_from_balance(Snapshot(balance))
                }
                for instrument in &snapshot.instruments {
                    self.instrument_mut(&instrument.instrument)
                        .update_from_account_snapshot(instrument)
                }
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.asset_mut(&balance.0.asset)
                    .update_from_balance(balance.as_ref());
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
        // Todo: set execution ConnectivityState to healthy if unhealthy
        let instrument_state = self.instrument_mut(&event.instrument);

        instrument_state.market.process(event);
        self.strategy.process(event);
        self.risk.process(event);
    }
}

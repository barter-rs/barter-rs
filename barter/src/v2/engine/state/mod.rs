use crate::v2::{
    engine::{
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
    },
    execution::{AccountEvent, AccountEventKind}
    ,
    Snapshot,
};
use barter_data::event::MarketEvent;
use barter_instrument::{
    asset::AssetIndex,
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

pub struct TradingStateUpdateAudit {
    pub prev: TradingState,
    pub current: TradingState,
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

        self.instrument_mut(&event.instrument).market.process(event);
        self.strategy.process(event);
        self.risk.process(event);
    }
}







use crate::v2::engine::Processor;
use crate::v2::{
    engine::state::balance::BalanceManager,
    execution::{AccountEvent, AccountEventKind},
};
use barter_data::event::MarketEvent;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;
use tracing::{debug, info, warn};
use instrument::position::PositionManager;
use crate::v2::engine::state::instrument::InstrumentStateManager;
use crate::v2::engine::state::instrument::market_data::MarketDataManager;
use crate::v2::engine::state::instrument::order::OrderManager;

pub mod balance;
pub mod instrument;

// pub trait UpdateFromSnapshot<Snapshot> {
//     fn update_from_snapshot(&mut self, snapshot: &Snapshot);
// }

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct EngineState<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey> {
    pub trading: TradingState,
    pub instruments: InstrumentState,
    pub balances: BalanceState,
    pub strategy: StrategyState,
    pub risk: RiskState,
    pub phantom: PhantomData<(AssetKey, InstrumentKey)>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum TradingState {
    Enabled,
    Disabled,
}

impl<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey> Processor<TradingState>
    for EngineState<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey> {
    type Output = ();

    fn process(&mut self, event: TradingState) -> Self::Output {
        let next = match (self.trading, event) {
            (TradingState::Enabled, TradingState::Disabled) => {
                info!("Engine disabled trading");
                TradingState::Disabled
            }
            (TradingState::Disabled, TradingState::Enabled) => {
                info!("Engine enabled trading");
                TradingState::Enabled
            }
            (TradingState::Enabled, TradingState::Enabled) => {
                info!("Engine enabled trading, although it was already enabled");
                TradingState::Enabled
            }
            (TradingState::Disabled, TradingState::Disabled) => {
                info!("Engine disabled trading, although it was already disabled");
                TradingState::Disabled
            }
        };

        self.trading = next;
    }
}

impl<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey>
    Processor<&AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
    for EngineState<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey>
where
    InstrumentState: InstrumentStateManager<InstrumentKey>,
    BalanceState: BalanceManager<AssetKey>,
    StrategyState: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>,
    RiskState: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>,
    AssetKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
{
    type Output = ();

    fn process(
        &mut self,
        event: &AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>,
    ) -> Self::Output {
        info!(
            account = ?event,
            "updating InstrumentState, BalanceState, RiskState, StrategyState from AccountEvent"
        );

        // Update InstrumentState & BalanceState
        let AccountEvent { exchange, kind } = event;
        match kind {
            AccountEventKind::Snapshot(account) => {
                self.instruments.update_from_snapshot(&account.instruments);
                self.balances.update_from_exchange_balance_snapshot(exchange, account.balances.as_ref());
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.balances.update_from_balance_snapshot(exchange, balance.as_ref());
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instruments.orders_mut().update_from_order_snapshot(order.as_ref());
            }
            AccountEventKind::PositionSnapshot(position) => {
                self.instruments.positions_mut().update_from_position_snapshot(position.as_ref());
            }
            AccountEventKind::OrderOpened(response) => {
                self.instruments.orders_mut().update_from_open(response);
            }
            AccountEventKind::OrderCancelled(response) => {
                self.instruments.orders_mut().update_from_cancel(response);
            }
            AccountEventKind::Trade(trade) => {
                self.instruments.positions_mut().update_from_trade(trade);
            }
            AccountEventKind::ConnectivityError(error) => {
                warn!(%error, "Engine aware of Account ConnectivityError");
            }
        }

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);
    }
}

impl<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey> Processor<&MarketEvent<InstrumentKey>>
    for EngineState<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey>
where
    InstrumentKey: Debug + Clone,
    InstrumentState: InstrumentStateManager<InstrumentKey>,
    StrategyState: for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
    RiskState: for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
{
    type Output = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey>) -> Self::Output {
        debug!(market = ?event, "updating EngineState, RiskState, StrategyState from MarketEvent");
        self.instruments.market_data_mut().update_from_market(event);

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);
    }
}

// impl<InstrumentState, BalanceState, StrategyState, RiskState, InstrumentKey, AssetKey> EngineState<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey>
// where
//     InstrumentState: InstrumentStateManager<InstrumentKey>,
//     BalanceState: BalanceManager<AssetKey>,
//     // AssetKey: Clone,
//     InstrumentKey: Debug// + Clone,
// {
//     /// Replace all [`Self`] state with the [`AccountSnapshot`].
//     ///
//     /// All open & cancel in-flight requests will be deleted.
//     pub fn update_from_account_snapshot(
//         &mut self,
//         exchange: &Exchange,
//         snapshot: &AccountSnapshot<AssetKey, InstrumentKey>,
//     ) {
//         let AccountSnapshot {
//             balances,
//             instruments,
//         } = snapshot;
//
//         // Update Balances
//         balances.iter().for_each(|asset_balance| {
//             self.instrument
//                 .balances_mut()
//                 .update_from_snapshot(exchange, Snapshot(asset_balance))
//         });
//
//         // Update InstrumentStates (Positions & Orders)
//         for snapshot in instruments {
//             let instrument = &snapshot.position.instrument;
//             if let Some(state) = self.instrument.state_mut(instrument) {
//                 let _ = std::mem::replace(&mut state.position, snapshot.position.clone());
//
//                 // Note: this wipes all open & cancel in-flight requests
//                 let _ = std::mem::replace(
//                     &mut state.orders.inner,
//                     snapshot
//                         .orders
//                         .iter()
//                         .map(|order| (order.cid, Order::from(order.clone())))
//                         .collect(),
//                 );
//             } else {
//                 warn!(
//                     ?instrument,
//                     event = ?snapshot,
//                     "EngineState ignoring InstrumentAccountSnapshot received for non-configured instrument"
//                 );
//             }
//         }
//     }
//
//     pub fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey>) {
//         self.instrument.market_data_mut().update_from_market(event);
//     }
// }

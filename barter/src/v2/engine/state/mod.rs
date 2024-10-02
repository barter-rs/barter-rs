use crate::v2::engine::Processor;
use crate::v2::{
    engine::state::{
        balance::{BalanceManager},
        instrument::{MarketDataManager, OrderManager, PositionManager},
    },
    execution::{AccountEvent, AccountEventKind, AccountSnapshot},
};
use barter_data::event::MarketEvent;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;
use tracing::{debug, info, warn};

pub mod balance;
pub mod instrument;

pub trait InstrumentStateManager<AssetKey, InstrumentKey>
where
    Self: Clone,
{
    fn update_from_snapshot(&mut self, snapshot: &AccountSnapshot<AssetKey, InstrumentKey>);
    fn market_data(&self) -> &impl MarketDataManager<InstrumentKey>;
    fn market_data_mut(&mut self) -> &mut impl MarketDataManager<InstrumentKey>;
    fn balances(&self) -> &impl BalanceManager<AssetKey>;
    fn balances_mut(&mut self) -> &mut impl BalanceManager<AssetKey>;
    fn orders(&self) -> &impl OrderManager<InstrumentKey>;
    fn orders_mut(&mut self) -> &mut impl OrderManager<InstrumentKey>;
    fn positions(&self) -> &impl PositionManager<InstrumentKey>;
    fn positions_mut(&mut self) -> &mut impl PositionManager<InstrumentKey>;
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct EngineState<InstrumentState, StrategyState, RiskState, AssetKey, InstrumentKey> {
    pub trading: TradingState,
    pub instrument: InstrumentState,
    pub strategy: StrategyState,
    pub risk: RiskState,
    phantom: PhantomData<(AssetKey, InstrumentKey)>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum TradingState {
    Enabled,
    Disabled,
}

impl<InstrumentState, StrategyState, RiskState, AssetKey, InstrumentKey> Processor<TradingState>
    for EngineState<InstrumentState, StrategyState, RiskState, AssetKey, InstrumentKey> {
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

impl<AssetKey, InstrumentKey, InstrumentState, StrategyState, RiskState>
    Processor<&AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
    for EngineState<InstrumentState, StrategyState, RiskState, AssetKey, InstrumentKey>
where
    AssetKey: Debug + Clone,
    InstrumentKey: Debug + Clone,
    InstrumentState: InstrumentStateManager<AssetKey, InstrumentKey>,
    StrategyState: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>,
    RiskState: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>,
{
    type Output = ();

    fn process(
        &mut self,
        event: &AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>,
    ) -> Self::Output {
        info!(account = ?event, "updating EngineState, RiskState, StrategyState from AccountEvent");
        self.update_from_account(event);

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);
    }
}

impl<AssetKey, InstrumentKey, InstrumentState, StrategyState, RiskState> Processor<&MarketEvent<InstrumentKey>>
    for EngineState<InstrumentState, StrategyState, RiskState, AssetKey, InstrumentKey>
where
    AssetKey: Clone,// + Eq,
    InstrumentKey: Debug + Clone,// + Eq,
    InstrumentState: InstrumentStateManager<AssetKey, InstrumentKey>,
    StrategyState: for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
    RiskState: for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
{
    type Output = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey>) -> Self::Output {
        debug!(market = ?event, "updating EngineState, RiskState, StrategyState from MarketEvent");
        self.instrument.market_data_mut().update_from_market(event);

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);
    }
}

impl<InstrumentState, StrategyState, RiskState, AssetKey, InstrumentKey> EngineState<InstrumentState, StrategyState, RiskState, AssetKey, InstrumentKey>
where
    InstrumentState: InstrumentStateManager<AssetKey, InstrumentKey>,
    // AssetKey: Clone,
    InstrumentKey: Debug// + Clone,
{
    pub fn update_from_account(
        &mut self,
        event: &AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>,
    ) {
        let AccountEvent { exchange, kind } = event;
        match kind {
            AccountEventKind::Snapshot(account) => {
                self.instrument.update_from_snapshot(account);
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.instrument.balances_mut().update_from_snapshot(exchange, balance.as_ref());
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instrument.orders_mut().update_from_order_snapshot(order);
            }
            AccountEventKind::PositionSnapshot(position) => {
                self.instrument.positions_mut().update_from_position_snapshot(position);
            }
            AccountEventKind::OrderOpened(response) => {
                self.instrument.orders_mut().update_from_open(response);
            }
            AccountEventKind::OrderCancelled(response) => {
                self.instrument.orders_mut().update_from_cancel(response);
            }
            AccountEventKind::Trade(trade) => {
                self.instrument.positions_mut().update_from_trade(trade);
            }
            AccountEventKind::ConnectivityError(error) => {
                warn!(%error, "Engine aware of Account ConnectivityError");
            }
        }
    }

    // /// Replace all [`Self`] state with the [`AccountSnapshot`].
    // ///
    // /// All open & cancel in-flight requests will be deleted.
    // pub fn update_from_account_snapshot(
    //     &mut self,
    //     exchange: &Exchange,
    //     snapshot: &AccountSnapshot<AssetKey, InstrumentKey>,
    // ) {
    //     let AccountSnapshot {
    //         balances,
    //         instruments,
    //     } = snapshot;
    //
    //     // Update Balances
    //     balances.iter().for_each(|asset_balance| {
    //         self.instrument
    //             .balances_mut()
    //             .update_from_snapshot(exchange, Snapshot(asset_balance))
    //     });
    //
    //     // Update InstrumentStates (Positions & Orders)
    //     for snapshot in instruments {
    //         let instrument = &snapshot.position.instrument;
    //         if let Some(state) = self.instrument.state_mut(instrument) {
    //             let _ = std::mem::replace(&mut state.position, snapshot.position.clone());
    //
    //             // Note: this wipes all open & cancel in-flight requests
    //             let _ = std::mem::replace(
    //                 &mut state.orders.inner,
    //                 snapshot
    //                     .orders
    //                     .iter()
    //                     .map(|order| (order.cid, Order::from(order.clone())))
    //                     .collect(),
    //             );
    //         } else {
    //             warn!(
    //                 ?instrument,
    //                 event = ?snapshot,
    //                 "EngineState ignoring InstrumentAccountSnapshot received for non-configured instrument"
    //             );
    //         }
    //     }
    // }
    //
    // pub fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey>) {
    //     self.instrument.market_data_mut().update_from_market(event);
    // }
}

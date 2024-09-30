use crate::v2::engine::Processor;
use crate::v2::{
    engine::state::{
        balance::{BalanceManager, Balances},
        instrument::{Instruments, MarketDataManager, OrderManager, PositionManager},
    },
    execution::{AccountEvent, AccountEventKind, AccountSnapshot},
    order::Order,
    Snapshot,
};
use barter_data::event::MarketEvent;
use barter_integration::model::Exchange;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{debug, info, warn};

pub mod balance;
pub mod instrument;

pub trait EngineState<AssetKey, InstrumentKey, StrategyState, RiskState> {
    fn trading_state(&self) -> TradingState;
    fn trading_state_mut(&mut self) -> &mut TradingState;
    fn market_data(&self) -> &impl MarketDataManager<InstrumentKey>;
    fn market_data_mut(&mut self) -> &mut impl MarketDataManager<InstrumentKey>;
    fn balances(&self) -> &impl BalanceManager<AssetKey>;
    fn balances_mut(&mut self) -> &mut impl BalanceManager<AssetKey>;
    fn orders(&self) -> &impl OrderManager<InstrumentKey>;
    fn orders_mut(&mut self) -> &mut impl OrderManager<InstrumentKey>;
    fn positions(&self) -> &impl PositionManager<InstrumentKey>;
    fn positions_mut(&mut self) -> &mut impl PositionManager<InstrumentKey>;
    fn strategy(&self) -> &StrategyState;
    fn strategy_mut(&mut self) -> &mut StrategyState;
    fn risk(&self) -> &RiskState;
    fn risk_mut(&mut self) -> &mut RiskState;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum TradingState {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Eq,
    InstrumentKey: Eq,
{
    pub trading: TradingState,
    pub balances: Balances<AssetKey>,
    pub instruments: Instruments<InstrumentKey>,
    pub strategy: StrategyState,
    pub risk: RiskState,
}

impl<AssetKey, InstrumentKey, StrategyState, RiskState>
    EngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
    for DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Debug + Eq,
    InstrumentKey: Debug + Eq + Clone,
{
    fn trading_state(&self) -> TradingState {
        self.trading
    }

    fn trading_state_mut(&mut self) -> &mut TradingState {
        &mut self.trading
    }

    fn market_data(&self) -> &impl MarketDataManager<InstrumentKey> {
        &self.instruments
    }

    fn market_data_mut(&mut self) -> &mut impl MarketDataManager<InstrumentKey> {
        &mut self.instruments
    }

    fn balances(&self) -> &impl BalanceManager<AssetKey> {
        &self.balances
    }

    fn balances_mut(&mut self) -> &mut impl BalanceManager<AssetKey> {
        &mut self.balances
    }

    fn orders(&self) -> &impl OrderManager<InstrumentKey> {
        &self.instruments
    }

    fn orders_mut(&mut self) -> &mut impl OrderManager<InstrumentKey> {
        &mut self.instruments
    }

    fn positions(&self) -> &impl PositionManager<InstrumentKey> {
        &self.instruments
    }

    fn positions_mut(&mut self) -> &mut impl PositionManager<InstrumentKey> {
        &mut self.instruments
    }

    fn strategy(&self) -> &StrategyState {
        &self.strategy
    }

    fn strategy_mut(&mut self) -> &mut StrategyState {
        &mut self.strategy
    }

    fn risk(&self) -> &RiskState {
        &self.risk
    }

    fn risk_mut(&mut self) -> &mut RiskState {
        &mut self.risk
    }
}

impl<AssetKey, InstrumentKey, StrategyState, RiskState> Processor<TradingState>
    for DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Debug + Eq,
    InstrumentKey: Debug + Clone + Eq,
{
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

        *self.trading_state_mut() = next;
    }
}

impl<AssetKey, InstrumentKey, StrategyState, RiskState>
    Processor<&AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
    for DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Debug + Eq,
    InstrumentKey: Debug + Clone + Eq,
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

impl<AssetKey, InstrumentKey, StrategyState, RiskState> Processor<&MarketEvent<InstrumentKey>>
    for DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Debug + Eq,
    InstrumentKey: Debug + Clone + Eq,
    StrategyState: for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
    RiskState: for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
{
    type Output = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey>) -> Self::Output {
        debug!(market = ?event, "updating EngineState, RiskState, StrategyState from MarketEvent");
        self.update_from_market(event);

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);
    }
}

impl<AssetKey, InstrumentKey, StrategyState, RiskState>
    DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Debug + Eq,
    InstrumentKey: Debug + Clone + Eq,
{
    pub fn update_from_account(
        &mut self,
        event: &AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>,
    ) {
        let AccountEvent { exchange, kind } = event;
        match kind {
            AccountEventKind::Snapshot(account) => {
                self.update_from_account_snapshot(exchange, account);
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.balances
                    .update_from_snapshot(exchange, balance.as_ref());
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instruments.update_from_order_snapshot(order);
            }
            AccountEventKind::PositionSnapshot(position) => {
                self.instruments.update_from_position_snapshot(position);
            }
            AccountEventKind::OrderOpened(response) => {
                self.instruments.update_from_open(response);
            }
            AccountEventKind::OrderCancelled(response) => {
                self.instruments.update_from_cancel(response);
            }
            AccountEventKind::Trade(trade) => {
                self.instruments.update_from_trade(trade);
            }
            AccountEventKind::ConnectivityError(error) => {
                warn!(%error, "Engine aware of Account ConnectivityError");
            }
        }
    }

    /// Replace all [`Self`] state with the [`AccountSnapshot`].
    ///
    /// All open & cancel in-flight requests will be deleted.
    pub fn update_from_account_snapshot(
        &mut self,
        exchange: &Exchange,
        snapshot: &AccountSnapshot<AssetKey, InstrumentKey>,
    ) {
        let AccountSnapshot {
            balances,
            instruments,
        } = snapshot;

        // Update Balances
        balances.iter().for_each(|asset_balance| {
            self.balances
                .update_from_snapshot(exchange, Snapshot(asset_balance))
        });

        // Update InstrumentStates (Positions & Orders)
        for snapshot in instruments {
            let instrument = &snapshot.position.instrument;
            if let Some(state) = self.instruments.state_mut(instrument) {
                let _ = std::mem::replace(&mut state.position, snapshot.position.clone());

                // Note: this wipes all open & cancel in-flight requests
                let _ = std::mem::replace(
                    &mut state.orders.inner,
                    snapshot
                        .orders
                        .iter()
                        .map(|order| (order.cid, Order::from(order.clone())))
                        .collect(),
                );
            } else {
                warn!(
                    ?instrument,
                    event = ?snapshot,
                    "EngineState ignoring InstrumentAccountSnapshot received for non-configured instrument"
                );
            }
        }
    }

    pub fn update_from_market(&mut self, event: &MarketEvent<InstrumentKey>) {
        self.instruments.update_from_market(event);
    }
}

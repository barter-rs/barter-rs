use crate::v2::{
    engine::{
        state::{balance::BalanceManager, instrument::order::OrderManager},
        Processor,
    },
    execution::{AccountEvent, AccountEventKind, InstrumentAccountSnapshot},
};
use barter_data::event::MarketEvent;
use derive_more::Constructor;
use instrument::position::PositionManager;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, marker::PhantomData};
use tracing::{debug, info, warn};

pub mod balance;
pub mod instrument;

pub trait UpdateFromKeyedSnapshot<Snapshot> {
    type Key;
    fn update_from_keyed_snapshot(&mut self, key: &Self::Key, snapshot: &Snapshot);
}

pub trait UpdateFromSnapshot<Snapshot> {
    fn update_from_snapshot(&mut self, snapshot: &Snapshot);
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct EngineState<
    InstrumentState,
    BalanceState,
    StrategyState,
    RiskState,
    AssetKey,
    InstrumentKey,
> {
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

impl<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey>
    Processor<TradingState>
    for EngineState<
        InstrumentState,
        BalanceState,
        StrategyState,
        RiskState,
        AssetKey,
        InstrumentKey,
    >
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

        self.trading = next;
    }
}

impl<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey>
    Processor<&AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
    for EngineState<
        InstrumentState,
        BalanceState,
        StrategyState,
        RiskState,
        AssetKey,
        InstrumentKey,
    >
where
    InstrumentState: UpdateFromSnapshot<Vec<InstrumentAccountSnapshot<InstrumentKey>>>
        + OrderManager<InstrumentKey>
        + PositionManager<AssetKey, InstrumentKey>,
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
            AccountEventKind::Snapshot(snapshot) => {
                self.balances
                    .update_from_keyed_snapshot(exchange, &snapshot.balances);
                self.instruments.update_from_snapshot(&snapshot.instruments);
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.balances
                    .update_from_balance(exchange, balance.as_ref());
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instruments.update_from_order(order.as_ref());
            }
            AccountEventKind::PositionSnapshot(position) => {
                self.instruments
                    .update_from_position_snapshot(position.as_ref());
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

        // Update any user provided Strategy & Risk State
        self.strategy.process(event);
        self.risk.process(event);
    }
}

impl<
        InstrumentState,
        BalanceState,
        StrategyState,
        RiskState,
        AssetKey,
        InstrumentKey,
        MarketDataKind,
    > Processor<&MarketEvent<InstrumentKey, MarketDataKind>>
    for EngineState<
        InstrumentState,
        BalanceState,
        StrategyState,
        RiskState,
        AssetKey,
        InstrumentKey,
    >
where
    InstrumentState: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketDataKind>>,
    StrategyState: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketDataKind>>,
    RiskState: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketDataKind>>,
    InstrumentKey: Debug + Clone,
    MarketDataKind: Debug,
{
    type Output = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey, MarketDataKind>) -> Self::Output {
        debug!(
            market = ?event,
            "updating InstrumentState, BalanceState, RiskState, StrategyState from MarketEvent"
        );

        self.instruments.process(event);
        self.strategy.process(event);
        self.risk.process(event);
    }
}

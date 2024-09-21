use crate::v2::engine::Processor;
use crate::v2::{
    engine::{
        error::EngineError,
        state::{
            balance::{BalanceManager, Balances},
            instrument::{Instruments, MarketDataManager, OrderManager, PositionManager},
        },
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

// Todo: Must Have:
//  - Utility to re-create state from Audit snapshot + updates w/ interactive mode (backward would require Vec<State> to be created on .next()) (add compression using file system)
//  - Basic Command functionality
//  - All state update implementations:
//  - Add tests for all Managers:
//  - Add interface for user strategy & risk to access Instrument contract
//  - Abstract AssetKey & InstrumentKey all the way up.
//  - Engine functionality can be injected, on_shutdown, on_state_update_error, on_disconnect, etc.
//  - Utility for AssetKey, InstrumentKey lookups, as well as constructing Instruments contracts, etc

// Todo: Nice To Have:
//  - Sequenced log stream that can enrich logs w/ additional context eg/ InstrumentName
//  - Consider removing duplicate logs when calling instrument.state, state_mut, and also Balances!
//  - Extract methods from impl OrderManager for Orders (eg/ update_from_snapshot covers all bases)
//    '--> also ensure duplication is removed from update_from_open & update_from_cancel
//  - Should I collapse nested VecMap in balances and use eg/ VecMap<ExchangeAssetKey, Balance>
//  - Setup some way to get "diffs" for eg/ should Orders.update_from_order_snapshot return a diff?
//  - Could use TradingState like concept to switch between Strategies / run loops

// Todo: Nice To Have: OrderManager:
//  - OrderManager update_from_open & update_from_cancel may want to return "in flight failed due to X api reason"
//    '--> eg/ find logic associated with "OrderManager received ExecutionError for Order<InFlight>"
//  - Possible we want a 5m window buffer for "strange order updates" to handle out of orders
//    '--> eg/ adding InFlight, receiving Cancelled, the receiving Open -> ghost orders

// Could have Generic command, with custom functionality for it:
// match command {
//    Terminate => engine.terminate(),
// etc. etc.
//
//

// pub trait EngineState<Event, AssetKey, InstrumentKey, StrategyState, RiskState>
pub trait EngineState<AssetKey, InstrumentKey, StrategyState, RiskState> {
    fn trading_enabled(&self) -> bool;
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Eq,
    InstrumentKey: Eq,
{
    pub trading_on: bool,
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
    // StrategyState: for<'a> StateUpdater<&'a EngineEvent, Output = (), Error = EngineError> + Debug + Clone,
    // RiskState: for<'a> StateUpdater<&'a EngineEvent, Output = (), Error = EngineError> + Debug + Clone,
    // StrategyState: for<'a> Processor<&'a EngineEvent> + Debug + Clone,
    // RiskState: for<'a> Processor<&'a EngineEvent> + Debug + Clone,
{
    fn trading_enabled(&self) -> bool {
        self.trading_on
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
        info!(account = ?event, "updating EngineState from AccountEvent");
        self.try_update_from_account(event).unwrap(); // Todo: handle this w/ audit, etc audit?

        // Update any user provided Strategy & Risk State
        self.strategy.process(event); // Todo: probably return an error, or perhaps some AuditKind?
        self.risk.process(event); // Todo: probably return an error, or perhaps some AuditKind?
    }
}

impl<AssetKey, InstrumentKey, StrategyState, RiskState> Processor<&MarketEvent<InstrumentKey>>
    for DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Debug + Eq,
    InstrumentKey: Debug + Clone + Eq,
    StrategyState: for<'a> Processor<&'a MarketEvent<InstrumentKey>>, //, Output = Result<(), EngineError>>,
    RiskState: for<'a> Processor<&'a MarketEvent<InstrumentKey>>, //, Output = Result<(), EngineError>>,
{
    type Output = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey>) -> Self::Output {
        debug!(market = ?event, "updating EngineState from MarketEvent");
        self.update_from_market(event); // Todo: should this return an error?

        // Update any user provided Strategy & Risk State
        self.strategy.process(event); // Todo: probably return an error, or perhaps some AuditKind?
        self.risk.process(event); // Todo: probably return an error, or perhaps some AuditKind?
    }
}

// impl<StrategyState, RiskState> StateUpdater<&EngineEvent> for DefaultEngineState<StrategyState, RiskState>
// where
//     StrategyState: for<'a> StateUpdater<&'a EngineEvent, Error = EngineError, Output = ()>,
//     RiskState: for<'a> StateUpdater<&'a EngineEvent, Error = EngineError, Output = ()>,
// {
//     type Output = ();
//     type Error = EngineError;
//
//     fn try_update(&mut self, event: &EngineEvent) -> Result<(), Self::Error> {
//         // Update core EngineState components
//         match event {
//             EngineEvent::Command(command) => {
//                 info!(?command, "updating EngineState from Command");
//                 self.update_from_command(command);
//             }
//             EngineEvent::Account(event) => {
//                 info!(account = ?event, "updating EngineState from AccountEvent");
//                 self.try_update_from_account(event)?
//             }
//             EngineEvent::Market(event) => {
//                 debug!(market = ?event, "updating EngineState from MarketEvent");
//                 self.update_from_market(event);
//             }
//         }
//
//         // Update any user provided Strategy & Risk State
//         self.strategy.try_update(event)?;
//         self.risk.try_update(event)
//     }
// }

impl<AssetKey, InstrumentKey, StrategyState, RiskState>
    DefaultEngineState<AssetKey, InstrumentKey, StrategyState, RiskState>
where
    AssetKey: Debug + Eq,
    InstrumentKey: Debug + Clone + Eq,
{
    pub fn update_from_command_enable_trading(&mut self) {
        if self.trading_on {
            info!("Engine enabled trading, although it was already enabled");
        } else {
            self.trading_on = true;
            info!("Engine enabled trading");
        }
    }

    pub fn update_from_command_disable_trading(&mut self) {
        if self.trading_on {
            self.trading_on = false;
            info!("Engine disabled trading");
        } else {
            info!("Engine disabled trading, although it was already disabled");
        }
    }

    pub fn try_update_from_account(
        &mut self,
        event: &AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>,
    ) -> Result<(), EngineError> {
        let AccountEvent { exchange, kind } = event;
        match kind {
            AccountEventKind::Snapshot(account) => {
                self.update_from_account_snapshot(exchange, account);
                Ok(())
            }
            AccountEventKind::BalanceSnapshot(balance) => {
                self.balances
                    .update_from_snapshot(exchange, balance.as_ref());
                Ok(())
            }
            AccountEventKind::OrderSnapshot(order) => {
                self.instruments.update_from_order_snapshot(order);
                Ok(())
            }
            AccountEventKind::PositionSnapshot(position) => {
                self.instruments.update_from_position_snapshot(position);
                Ok(())
            }
            AccountEventKind::OrderOpened(response) => {
                self.instruments.update_from_open(response);
                Ok(())
            }
            AccountEventKind::OrderCancelled(response) => {
                self.instruments.update_from_cancel(response);
                Ok(())
            }
            AccountEventKind::Trade(trade) => {
                self.instruments.update_from_trade(trade);
                Ok(())
            }
            AccountEventKind::ConnectivityError(error) => {
                warn!(%error, "Engine aware of Account ConnectivityError");
                Ok(())
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

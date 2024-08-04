use crate::v2::{
    engine::{
        command::Command,
        error::EngineError,
        state::{
            balance::{BalanceManager, Balances},
            instrument::{Instruments, MarketDataManager, OrderManager, PositionManager},
        },
    },
    execution::{AccountEvent, AccountEventKind, AccountSnapshot},
    instrument::asset::AssetId,
    order::Order,
    EngineEvent, Snapshot, TryUpdater,
};
use barter_data::{event::MarketEvent, instrument::InstrumentId};
use barter_integration::model::Exchange;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{debug, info, warn};

pub mod balance;
pub mod instrument;

// Todo:
//  - Setup some way to get "diffs" for eg/ should Orders.update_from_order_snapshot return a diff?
//  - Should I collapse nested VecMap in balances and use eg/ VecMap<ExchangeAssetKey, Balance>
//  - Add tests for all Managers, especially Orders & Positions!
//  - Consider removing duplicate logs when calling instrument.state, state_mut, and also Balances!
//  - Consider adding Error generics/assoc types to improve flexibility of Strategy & Risk managers
//  - Extract methods from impl OrderManager for Orders (eg/ update_from_snapshot covers all bases)
//    '--> also ensure duplication is removed from update_from_open & update_from_cancel
//  - Make EngineError more generic to add flexibility to user to define their own
//  - Allow users to perform shutdown tasks
//  - Add interface for user Strategy & Risk to access Instrument contract
//  - EngineState should have assoc types for AssetKey & InstrumentKey, to pass to Strategy & Risk?

// Todo: OrderManager:
//  - OrderManager update_from_open & update_from_cancel may want to return "in flight failed due to X api reason"
//    '--> eg/ find logic associated with "OrderManager received ExecutionError for Order<InFlight>"

pub trait EngineState<Event, AssetKey, InstrumentKey, StrategyState, RiskState>
where
    Self: for<'a> TryUpdater<&'a Event> + Debug + Clone,
    StrategyState: for<'a> TryUpdater<&'a Event> + Debug + Clone,
    RiskState: for<'a> TryUpdater<&'a Event> + Debug + Clone,
{
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
pub struct DefaultEngineState<StrategyState, RiskState> {
    pub balances: Balances,
    pub instruments: Instruments,
    pub strategy: StrategyState,
    pub risk: RiskState,
}

impl<StrategyState, RiskState>
    EngineState<EngineEvent, AssetId, InstrumentId, StrategyState, RiskState>
    for DefaultEngineState<StrategyState, RiskState>
where
    StrategyState: for<'a> TryUpdater<&'a EngineEvent, Error = EngineError> + Debug + Clone,
    RiskState: for<'a> TryUpdater<&'a EngineEvent, Error = EngineError> + Debug + Clone,
{
    fn market_data(&self) -> &impl MarketDataManager<InstrumentId> {
        &self.instruments
    }

    fn market_data_mut(&mut self) -> &mut impl MarketDataManager<InstrumentId> {
        &mut self.instruments
    }

    fn balances(&self) -> &impl BalanceManager<AssetId> {
        &self.balances
    }

    fn balances_mut(&mut self) -> &mut impl BalanceManager<AssetId> {
        &mut self.balances
    }

    fn orders(&self) -> &impl OrderManager<InstrumentId> {
        &self.instruments
    }

    fn orders_mut(&mut self) -> &mut impl OrderManager<InstrumentId> {
        &mut self.instruments
    }

    fn positions(&self) -> &impl PositionManager<InstrumentId> {
        &self.instruments
    }

    fn positions_mut(&mut self) -> &mut impl PositionManager<InstrumentId> {
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

impl<StrategyState, RiskState> TryUpdater<&EngineEvent>
    for DefaultEngineState<StrategyState, RiskState>
where
    StrategyState: for<'a> TryUpdater<&'a EngineEvent, Error = EngineError>,
    RiskState: for<'a> TryUpdater<&'a EngineEvent, Error = EngineError>,
{
    type Error = EngineError;

    fn try_update(&mut self, event: &EngineEvent) -> Result<(), Self::Error> {
        // Update core EngineState components
        match event {
            EngineEvent::Command(command) => {
                info!(?command, "updating EngineState from Command");
                self.update_from_command(command);
            }
            EngineEvent::Account(event) => {
                info!(account = ?event, "updating EngineState from AccountEvent");
                self.try_update_from_account(event)?
            }
            EngineEvent::Market(event) => {
                debug!(market = ?event, "updating EngineState from MarketEvent");
                self.update_from_market(event);
            }
        }

        // Update any user provided Strategy & Risk State
        self.strategy.try_update(event)?;
        self.risk.try_update(event)
    }
}

impl<StrategyState, RiskState> DefaultEngineState<StrategyState, RiskState> {
    pub fn update_from_command(&mut self, command: &Command) {
        todo!()
    }

    pub fn try_update_from_account(
        &mut self,
        event: &AccountEvent<AccountEventKind<AssetId, InstrumentId>>,
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
                // Todo: handle error
                panic!("{}", error)
            }
        }
    }

    /// Replace all [`Self`] state with the [`AccountSnapshot`].
    ///
    /// All open & cancel in-flight requests will be deleted.
    pub fn update_from_account_snapshot(
        &mut self,
        exchange: &Exchange,
        snapshot: &AccountSnapshot<AssetId, InstrumentId>,
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
            let instrument = snapshot.position.instrument;
            if let Some(state) = self.instruments.state_mut(&instrument) {
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

    pub fn update_from_market(&mut self, event: &MarketEvent<InstrumentId>) {
        self.instruments.update_from_market(event);
    }
}

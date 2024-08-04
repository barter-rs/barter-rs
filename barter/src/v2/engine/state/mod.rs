use crate::v2::{
    engine::{
        state::{
            asset::{AssetState, AssetStates},
            connectivity::{Connection, ConnectivityState, ConnectivityStates},
            instrument::{InstrumentState, InstrumentStates},
            order_manager::OrderManager,
            trading::TradingState,
        },
        Processor,
    },
    execution::{manager::AccountStreamEvent, AccountEvent, AccountEventKind},
    Snapshot,
};
use barter_data::{event::MarketEvent, streams::consumer::MarketStreamEvent};
use barter_instrument::{
    asset::{name::AssetNameInternal, AssetIndex, ExchangeAsset},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{name::InstrumentNameInternal, InstrumentIndex},
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{info, warn};

pub mod asset;
pub mod connectivity;
pub mod instrument;
pub mod order_manager;
pub mod trading;

// Todo:
//  - Maybe introduce State machine for dealing with connectivity VecMap issue...
//    '--> could only check if a new Account/Market event updates to Connected if we are in
//         State=Unhealthy, that way we are only doing expensive lookup in that case
//  - Need to make some Key decisions about "what is a manager", and "what is an Updater"

// Todo: Consider splitting AccountEvents into AccountInstrumentEvents, AccountAssetEvent, Other
//       '--> ideally I can flip Update<AccountEvent> upside down to not duplicate logic
//       '--> issue becomes more impl Updater for user Strategy & Risk :(

pub type IndexedEngineState<Market, Strategy, Risk> =
    EngineState<Market, Strategy, Risk, ExchangeIndex, AssetIndex, InstrumentIndex>;

pub trait StateManager<Key> {
    type State;
    fn state(&self, key: &Key) -> &Self::State;
    fn state_mut(&mut self, key: &Key) -> &mut Self::State;
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
    EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentKey>
{
    pub fn update_from_trading_state_update(&mut self, event: &TradingState) {
        let next = match (self.trading, event) {
            (TradingState::Enabled, TradingState::Disabled) => {
                info!("EngineState setting TradingState::Disabled");
                TradingState::Disabled
            }
            (TradingState::Disabled, TradingState::Enabled) => {
                info!("EngineState setting TradingState::Enabled");
                TradingState::Enabled
            }
            (TradingState::Enabled, TradingState::Enabled) => {
                info!("EngineState set TradingState::Enabled, although it was already enabled");
                TradingState::Enabled
            }
            (TradingState::Disabled, TradingState::Disabled) => {
                info!("EngineState set TradingState::Disabled, although it was already disabled");
                TradingState::Disabled
            }
        };

        self.trading = next;
    }

    pub fn update_from_account(
        &mut self,
        event: &AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey>,
    ) where
        Self: StateManager<ExchangeId, State = ConnectivityState>
            + StateManager<AssetKey, State = AssetState>
            + StateManager<
                InstrumentKey,
                State = InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>,
            >,
        Market: Debug,
        Strategy: for<'a> Processor<&'a AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>,
        Risk: for<'a> Processor<&'a AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>,
        ExchangeKey: Debug + Clone,
        AssetKey: Debug,
        InstrumentKey: Debug + Clone,
    {
        match event {
            AccountStreamEvent::Reconnecting(exchange) => {
                warn!(
                    ?exchange,
                    "EngineState received AccountStream disconnected event"
                );
                self.state_mut(exchange).account = Connection::Reconnecting;
            }
            AccountStreamEvent::Item(event) => {
                info!(
                    account = ?event,
                    "EngineState updating from AccountEvent"
                );
                // Todo: set exchange ConnectivityState to healthy if unhealthy
                match &event.kind {
                    AccountEventKind::Snapshot(snapshot) => {
                        for balance in &snapshot.balances {
                            self.state_mut(&balance.asset)
                                .update_from_balance(Snapshot(balance))
                        }
                        for instrument in &snapshot.instruments {
                            self.state_mut(&instrument.position.instrument)
                                .update_from_account_snapshot(instrument)
                        }
                    }
                    AccountEventKind::BalanceSnapshot(balance) => {
                        self.state_mut(&balance.0.asset)
                            .update_from_balance(balance.as_ref());
                    }
                    AccountEventKind::PositionSnapshot(position) => {
                        self.state_mut(&position.0.instrument)
                            .update_from_position_snapshot(position.as_ref());
                    }
                    AccountEventKind::OrderSnapshot(order) => self
                        .state_mut(&order.0.instrument)
                        .orders
                        .update_from_order_snapshot(order.as_ref()),
                    AccountEventKind::OrderOpened(response) => self
                        .state_mut(&response.instrument)
                        .orders
                        .update_from_open(response),
                    AccountEventKind::OrderCancelled(response) => self
                        .state_mut(&response.instrument)
                        .orders
                        .update_from_cancel(response),
                    AccountEventKind::Trade(trade) => {
                        self.state_mut(&trade.instrument).update_from_trade(trade);
                    }
                }

                // Update any user provided Strategy & Risk State
                self.strategy.process(event);
                self.risk.process(event);
            }
        }
    }

    pub fn update_from_market<MarketEventKind>(
        &mut self,
        event: &MarketStreamEvent<InstrumentKey, MarketEventKind>,
    ) where
        Self: StateManager<ExchangeId, State = ConnectivityState>
            + StateManager<
                InstrumentKey,
                State = InstrumentState<Market, ExchangeKey, AssetKey, InstrumentKey>,
            >,
        Market: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketEventKind>>,
        Strategy: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketEventKind>>,
        Risk: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketEventKind>>,
    {
        match event {
            MarketStreamEvent::Reconnecting(exchange) => {
                warn!(
                    ?exchange,
                    "EngineState received MarketStream disconnected event"
                );
                self.state_mut(exchange).market_data = Connection::Reconnecting;
            }
            MarketStreamEvent::Item(event) => {
                // Todo: set exchange ConnectivityState to healthy if unhealthy
                self.state_mut(&event.instrument).market.process(event);
                self.strategy.process(event);
                self.risk.process(event);
            }
        }
    }
}

impl<Market, Strategy, Risk, AssetKey, InstrumentKey> StateManager<ExchangeIndex>
    for EngineState<Market, Strategy, Risk, ExchangeIndex, AssetKey, InstrumentKey>
{
    type State = ConnectivityState;

    fn state(&self, key: &ExchangeIndex) -> &Self::State {
        self.connectivity
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn state_mut(&mut self, key: &ExchangeIndex) -> &mut Self::State {
        self.connectivity
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk> StateManager<ExchangeId>
    for IndexedEngineState<Market, Strategy, Risk>
{
    type State = ConnectivityState;

    fn state(&self, key: &ExchangeId) -> &Self::State {
        self.connectivity
            .0
            .get(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }

    fn state_mut(&mut self, key: &ExchangeId) -> &mut Self::State {
        self.connectivity
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("ConnectivityStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk> StateManager<InstrumentIndex>
    for IndexedEngineState<Market, Strategy, Risk>
{
    type State = InstrumentState<Market, ExchangeIndex, AssetIndex, InstrumentIndex>;

    fn state(&self, key: &InstrumentIndex) -> &Self::State {
        self.instruments
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    fn state_mut(&mut self, key: &InstrumentIndex) -> &mut Self::State {
        self.instruments
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk> StateManager<InstrumentNameInternal>
    for EngineState<Market, Strategy, Risk, ExchangeId, AssetNameInternal, InstrumentNameInternal>
{
    type State = InstrumentState<Market, ExchangeId, AssetNameInternal, InstrumentNameInternal>;

    fn state(&self, key: &InstrumentNameInternal) -> &Self::State {
        self.instruments
            .0
            .get(key)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    fn state_mut(&mut self, key: &InstrumentNameInternal) -> &mut Self::State {
        self.instruments
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk> StateManager<AssetIndex>
    for IndexedEngineState<Market, Strategy, Risk>
{
    type State = AssetState;

    fn state(&self, key: &AssetIndex) -> &Self::State {
        self.assets
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }

    fn state_mut(&mut self, key: &AssetIndex) -> &mut Self::State {
        self.assets
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk> StateManager<ExchangeAsset<AssetNameInternal>>
    for EngineState<Market, Strategy, Risk, ExchangeId, AssetNameInternal, InstrumentNameInternal>
{
    type State = AssetState;

    fn state(&self, key: &ExchangeAsset<AssetNameInternal>) -> &Self::State {
        self.assets
            .0
            .get(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }

    fn state_mut(&mut self, key: &ExchangeAsset<AssetNameInternal>) -> &mut Self::State {
        self.assets
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("AssetStates does not contain: {key:?}"))
    }
}

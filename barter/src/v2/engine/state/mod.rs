use crate::v2::{
    engine::{
        state::{
            asset::{manager::AssetStateManager, AssetState, AssetStates},
            connectivity::{
                manager::ConnectivityManager, Connection, ConnectivityState, ConnectivityStates,
            },
            instrument::{
                manager::{InstrumentFilter, InstrumentStateManager},
                InstrumentState, InstrumentStates,
            },
            order::manager::OrderManager,
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
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{info, warn};

pub mod asset;
pub mod connectivity;
pub mod instrument;
pub mod order;
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
        let prev = self.trading;
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

        // ProcessTradingStateAudit {
        //     prev,
        //     current: next,
        // }
    }

    pub fn update_from_account(
        &mut self,
        event: &AccountEvent<ExchangeKey, AssetKey, InstrumentKey>,
    ) where
        Self: ConnectivityManager<ExchangeId>
            + AssetStateManager<AssetKey>
            + InstrumentStateManager<InstrumentKey, ExchangeKey = ExchangeKey, AssetKey = AssetKey>,
        Market: Debug,
        Strategy: for<'a> Processor<&'a AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>,
        Risk: for<'a> Processor<&'a AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>,
        ExchangeKey: Debug + Clone,
        AssetKey: Debug,
        InstrumentKey: Debug + Clone,
    {
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

    pub fn update_from_market<MarketEventKind>(
        &mut self,
        event: &MarketEvent<InstrumentKey, MarketEventKind>,
    ) where
        Self: ConnectivityManager<ExchangeId>
            + InstrumentStateManager<InstrumentKey, Market = Market>,
        Market: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketEventKind>>,
        Strategy: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketEventKind>>,
        Risk: for<'a> Processor<&'a MarketEvent<InstrumentKey, MarketEventKind>>,
    {
        // Todo: set exchange ConnectivityState to healthy if unhealthy
        self.instrument_mut(&event.instrument).market.process(event);
        self.strategy.process(event);
        self.risk.process(event);
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

impl<Market, Strategy, Risk, AssetKey, InstrumentKey> ConnectivityManager<ExchangeId>
    for EngineState<Market, Strategy, Risk, ExchangeId, AssetKey, InstrumentKey>
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

impl<Market, Strategy, Risk, ExchangeKey, AssetKey> InstrumentStateManager<InstrumentIndex>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentIndex>
{
    type ExchangeKey = ExchangeKey;
    type AssetKey = AssetKey;
    type Market = Market;

    fn instrument(
        &self,
        key: &InstrumentIndex,
    ) -> &InstrumentState<Market, ExchangeKey, AssetKey, InstrumentIndex> {
        self.instruments
            .0
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    fn instrument_mut(
        &mut self,
        key: &InstrumentIndex,
    ) -> &mut InstrumentState<Market, ExchangeKey, AssetKey, InstrumentIndex> {
        self.instruments
            .0
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }
}

impl<Market, Strategy, Risk, ExchangeKey, AssetKey> InstrumentStateManager<InstrumentNameInternal>
    for EngineState<Market, Strategy, Risk, ExchangeKey, AssetKey, InstrumentNameInternal>
{
    type ExchangeKey = ExchangeKey;
    type AssetKey = AssetKey;
    type Market = Market;

    fn instrument(
        &self,
        key: &InstrumentNameInternal,
    ) -> &InstrumentState<Market, ExchangeKey, AssetKey, InstrumentNameInternal> {
        self.instruments
            .0
            .get(key)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }

    fn instrument_mut(
        &mut self,
        key: &InstrumentNameInternal,
    ) -> &mut InstrumentState<Market, ExchangeKey, AssetKey, InstrumentNameInternal> {
        self.instruments
            .0
            .get_mut(key)
            .unwrap_or_else(|| panic!("InstrumentStates does not contain: {key}"))
    }
}

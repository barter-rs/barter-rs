use crate::{
    engine::{
        Engine,
        state::{
            EngineState,
            instrument::{data::InstrumentDataState, filter::InstrumentFilter},
        },
    },
    strategy::{
        algo::AlgoStrategy,
        close_positions::{ClosePositionsStrategy, close_open_positions_with_market_orders},
        on_disconnect::OnDisconnectStrategy,
        on_trading_disabled::OnTradingDisabled,
    },
};
use barter_execution::order::{
    id::{ClientOrderId, StrategyId},
    request::{OrderRequestCancel, OrderRequestOpen},
};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
};
use std::marker::PhantomData;

/// Defines a strategy interface for generating algorithmic open and cancel order requests based
/// on the current `EngineState`.
pub mod algo;

/// Defines a strategy interface for generating open and cancel order requests that close open
/// positions.
pub mod close_positions;

/// Defines a strategy interface enables custom [`Engine`] to be performed in the event of an
/// exchange disconnection.
pub mod on_disconnect;

/// Defines a strategy interface enables custom [`Engine`] to be performed in the event that the
/// `TradingState` gets set to `TradingState::Disabled`.
pub mod on_trading_disabled;

/// Naive implementation of all strategy interfaces.
///
/// *THIS IS FOR DEMONSTRATION PURPOSES ONLY, NEVER USE FOR REAL TRADING OR IN PRODUCTION*.
///
/// This strategy:
/// - Generates no algorithmic orders (AlgoStrategy).
/// - Closes positions via the naive [`close_open_positions_with_market_orders`] logic (ClosePositionsStrategy).
/// - Does nothing when an exchange disconnects (OnDisconnectStrategy).
/// - Does nothing when trading state is set to disabled (OnDisconnectStrategy).
#[derive(Debug, Clone)]
pub struct DefaultStrategy<State> {
    pub id: StrategyId,
    phantom: PhantomData<State>,
}

impl<State> Default for DefaultStrategy<State> {
    fn default() -> Self {
        Self {
            id: StrategyId::new("default"),
            phantom: PhantomData,
        }
    }
}

impl<State, ExchangeKey, InstrumentKey> AlgoStrategy<ExchangeKey, InstrumentKey>
    for DefaultStrategy<State>
{
    type State = State;

    fn generate_algo_orders(
        &self,
        _: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeKey, InstrumentKey>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeKey, InstrumentKey>>,
    ) {
        (std::iter::empty(), std::iter::empty())
    }
}

impl<GlobalData, InstrumentData> ClosePositionsStrategy
    for DefaultStrategy<EngineState<GlobalData, InstrumentData>>
where
    InstrumentData: InstrumentDataState,
{
    type State = EngineState<GlobalData, InstrumentData>;

    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>> + 'a,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>> + 'a,
    )
    where
        ExchangeIndex: 'a,
        AssetIndex: 'a,
        InstrumentIndex: 'a,
    {
        close_open_positions_with_market_orders(&self.id, state, filter, |_| {
            ClientOrderId::random()
        })
    }
}

impl<Clock, State, ExecutionTxs, Risk> OnDisconnectStrategy<Clock, State, ExecutionTxs, Risk>
    for DefaultStrategy<State>
{
    type OnDisconnect = ();

    fn on_disconnect(
        _: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
        _: ExchangeId,
    ) -> Self::OnDisconnect {
    }
}

impl<Clock, State, ExecutionTxs, Risk> OnTradingDisabled<Clock, State, ExecutionTxs, Risk>
    for DefaultStrategy<State>
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled {
    }
}

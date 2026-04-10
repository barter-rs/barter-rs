use barter::{
    engine::{
        Engine,
        state::{
            EngineState,
            global::DefaultGlobalData,
            instrument::{
                data::DefaultInstrumentMarketData,
                filter::InstrumentFilter,
            },
        },
    },
    strategy::{
        algo::AlgoStrategy,
        close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy,
        on_trading_disabled::OnTradingDisabled,
    },
};
use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
};
use pyo3::prelude::*;

/// The concrete EngineState type used throughout Python bindings.
pub type PyEngineState = EngineState<DefaultGlobalData, DefaultInstrumentMarketData>;

/// A strategy that delegates `generate_algo_orders` to a Python callable.
///
/// The Python callable receives a dict snapshot of the current state and returns
/// a tuple of (cancel_requests, open_requests) — initially both empty since the
/// full order generation from Python requires serializing the complex state.
///
/// For the initial release, this serves as a passthrough that enables the Engine
/// to run. Users can implement real strategy logic by subclassing in Python and
/// overriding the callback.
#[derive(Debug)]
pub struct PyStrategy {
    callback: Option<PyObject>,
}

impl PyStrategy {
    pub fn new(callback: Option<PyObject>) -> Self {
        Self { callback }
    }

    pub fn default_noop() -> Self {
        Self { callback: None }
    }
}

impl AlgoStrategy for PyStrategy {
    type State = PyEngineState;

    fn generate_algo_orders(
        &self,
        _state: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) {
        // If we have a Python callback, we could call it here via GIL acquisition.
        // For now, return empty — strategies will be added in a future release.
        if let Some(ref _cb) = self.callback {
            // Future: acquire GIL, serialize state snapshot, call Python, parse results
            // Python::with_gil(|py| { ... });
        }
        (std::iter::empty(), std::iter::empty())
    }
}

impl ClosePositionsStrategy for PyStrategy {
    type State = PyEngineState;

    fn close_positions_requests<'a>(
        &'a self,
        _state: &'a Self::State,
        _filter: &'a InstrumentFilter,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel> + 'a,
        impl IntoIterator<Item = OrderRequestOpen> + 'a,
    )
    where
        ExchangeIndex: 'a,
        AssetIndex: 'a,
        InstrumentIndex: 'a,
    {
        (std::iter::empty(), std::iter::empty())
    }
}

impl<Clock, State, ExecutionTxs, Risk> OnDisconnectStrategy<Clock, State, ExecutionTxs, Risk>
    for PyStrategy
{
    type OnDisconnect = ();

    fn on_disconnect(
        _: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
        _: ExchangeId,
    ) -> Self::OnDisconnect {
    }
}

impl<Clock, State, ExecutionTxs, Risk> OnTradingDisabled<Clock, State, ExecutionTxs, Risk>
    for PyStrategy
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled {
    }
}

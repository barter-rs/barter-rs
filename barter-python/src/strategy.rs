use crate::global::PyGlobalData;
use crate::instrument_data::PyInstrumentData;
use crate::order::{PyOrderRequestCancel, PyOrderRequestOpen};
use crate::state::PyEngineStateSnapshot;
use barter::{
    engine::{
        Engine,
        state::{
            EngineState,
            instrument::filter::InstrumentFilter,
        },
    },
    strategy::{
        algo::AlgoStrategy,
        close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy,
        on_trading_disabled::OnTradingDisabled,
    },
};
use barter_execution::order::{
    id::StrategyId,
    request::{OrderRequestCancel, OrderRequestOpen},
};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
};
use pyo3::prelude::*;

/// The concrete EngineState type used throughout Python bindings.
pub type PyEngineState = EngineState<PyGlobalData, PyInstrumentData>;

/// A strategy that delegates `generate_algo_orders` to one or more Python callables.
///
/// - Single strategy: one callback, orders tagged with `strategy_id="default"`
/// - Multi strategy: multiple named callbacks, each tags its own orders
#[derive(Debug)]
pub struct PyStrategy {
    /// Named strategy callbacks: (strategy_id, python_callable)
    callbacks: Vec<(StrategyId, PyObject)>,
}

impl PyStrategy {
    /// Single strategy mode.
    pub fn new(callback: Option<PyObject>) -> Self {
        let callbacks = match callback {
            Some(cb) => vec![(StrategyId::new("default"), cb)],
            None => vec![],
        };
        Self { callbacks }
    }

    /// Multi-strategy mode.
    pub fn new_multi(callbacks: Vec<(StrategyId, PyObject)>) -> Self {
        Self { callbacks }
    }

    pub fn default_noop() -> Self {
        Self { callbacks: vec![] }
    }
}

fn call_strategy_callback(
    py: Python<'_>,
    cb: &PyObject,
    strategy_id: &StrategyId,
    snapshot: &PyEngineStateSnapshot,
) -> (
    Vec<OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
    Vec<OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
) {
    let py_result = match cb.call1(py, (snapshot.clone(),)) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Python strategy '{}' callback error: {e}", strategy_id.0);
            return (Vec::new(), Vec::new());
        }
    };

    // Parse: (cancels, opens) or just opens
    let tuple = match py_result
        .extract::<(Vec<PyOrderRequestCancel>, Vec<PyOrderRequestOpen>)>(py)
    {
        Ok(t) => t,
        Err(_) => match py_result.extract::<Vec<PyOrderRequestOpen>>(py) {
            Ok(opens) => (Vec::new(), opens),
            Err(e) => {
                eprintln!(
                    "Python strategy '{}' must return List[OrderRequestOpen] \
                     or (List[OrderRequestCancel], List[OrderRequestOpen]): {e}",
                    strategy_id.0
                );
                return (Vec::new(), Vec::new());
            }
        },
    };

    // Override strategy_id on each order to match the callback's name
    let cancels = tuple.0.iter().map(|c| {
        let mut r = c.to_rust();
        r.key.strategy = strategy_id.clone();
        r
    }).collect();
    let opens = tuple.1.iter().map(|o| {
        let mut r = o.to_rust();
        r.key.strategy = strategy_id.clone();
        r
    }).collect();

    (cancels, opens)
}

impl AlgoStrategy for PyStrategy {
    type State = PyEngineState;

    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) {
        if self.callbacks.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let snapshot = PyEngineStateSnapshot::from_state(state);

        Python::with_gil(|py| {
            let mut all_cancels = Vec::new();
            let mut all_opens = Vec::new();

            for (strategy_id, cb) in &self.callbacks {
                let (cancels, opens) =
                    call_strategy_callback(py, cb, strategy_id, &snapshot);
                all_cancels.extend(cancels);
                all_opens.extend(opens);
            }

            (all_cancels, all_opens)
        })
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

use crate::order::{PyOrderRequestCancel, PyOrderRequestOpen};
use crate::state::PyEngineStateSnapshot;
use crate::strategy::PyEngineState;
use barter::risk::{RiskApproved, RiskManager, RiskRefused};
use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use pyo3::prelude::*;

/// A risk manager that delegates to a Python callable, or approves all orders.
///
/// The Python callable receives `(state, List[OrderRequestOpen])` and must
/// return a `List[OrderRequestOpen]` of approved orders (filtered subset).
///
/// If no callback is provided, all orders are approved (default behaviour).
#[derive(Debug)]
pub struct PyRiskManager {
    callback: Option<PyObject>,
}

impl PyRiskManager {
    pub fn new(callback: Option<PyObject>) -> Self {
        Self { callback }
    }

    pub fn default_noop() -> Self {
        Self { callback: None }
    }
}

impl RiskManager for PyRiskManager {
    type State = PyEngineState;

    fn check(
        &self,
        state: &Self::State,
        cancels: impl IntoIterator<Item = OrderRequestCancel>,
        opens: impl IntoIterator<Item = OrderRequestOpen>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<OrderRequestCancel>>,
        impl IntoIterator<Item = RiskApproved<OrderRequestOpen>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestCancel>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestOpen>>,
    ) {
        // Always approve cancels
        let approved_cancels: Vec<_> = cancels.into_iter().map(RiskApproved::new).collect();
        let all_opens: Vec<OrderRequestOpen> = opens.into_iter().collect();

        let Some(ref cb) = self.callback else {
            // No callback — approve everything
            let approved_opens: Vec<_> = all_opens.into_iter().map(RiskApproved::new).collect();
            let refused_opens: Vec<RiskRefused<OrderRequestOpen>> = vec![];
            return (approved_cancels, approved_opens, vec![], refused_opens);
        };

        // Build snapshot and Python-friendly order list
        let snapshot = PyEngineStateSnapshot::from_state(state);

        let (approved_opens, refused_opens) = Python::with_gil(|py| {
            // Convert opens to PyOrderRequestOpen for the callback
            let py_opens: Vec<PyOrderRequestOpen> = all_opens
                .iter()
                .map(|o| PyOrderRequestOpen {
                    exchange_index: o.key.exchange.0,
                    instrument_index: o.key.instrument.0,
                    strategy_id: o.key.strategy.0.to_string(),
                    side: match o.state.side {
                        barter_instrument::Side::Buy => "buy".to_string(),
                        barter_instrument::Side::Sell => "sell".to_string(),
                    },
                    price: o.state.price,
                    quantity: o.state.quantity,
                    order_kind: match o.state.kind {
                        barter_execution::order::OrderKind::Market => "market".to_string(),
                        barter_execution::order::OrderKind::Limit => "limit".to_string(),
                    },
                    time_in_force: "ioc".to_string(),
                })
                .collect();

            let result = match cb.call1(py, (snapshot, py_opens)) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Python risk callback error: {e}");
                    // On error, approve everything to avoid silently blocking
                    let approved: Vec<_> = all_opens.into_iter().map(RiskApproved::new).collect();
                    return (approved, Vec::new());
                }
            };

            // Expect a list of approved PyOrderRequestOpen (the filtered subset)
            match result.extract::<Vec<PyOrderRequestOpen>>(py) {
                Ok(approved_py) => {
                    // Convert approved Python orders back to Rust
                    let approved_rust: Vec<_> = approved_py
                        .iter()
                        .map(|o| RiskApproved::new(o.to_rust()))
                        .collect();
                    // Everything not returned is refused
                    // For simplicity, we don't track exact refused orders; the engine
                    // only executes approved ones, so refused is empty here
                    (approved_rust, Vec::new())
                }
                Err(e) => {
                    eprintln!(
                        "Python risk callback must return List[OrderRequestOpen]: {e}"
                    );
                    let approved: Vec<_> = all_opens.into_iter().map(RiskApproved::new).collect();
                    (approved, Vec::new())
                }
            }
        });

        (approved_cancels, approved_opens, vec![], refused_opens)
    }
}

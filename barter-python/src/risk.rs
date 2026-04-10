use crate::strategy::PyEngineState;
use barter::risk::{RiskApproved, RiskManager, RiskRefused};
use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use pyo3::prelude::*;

/// A risk manager that delegates to a Python callable, or uses DefaultRiskManager.
///
/// For the initial release, this wraps `DefaultRiskManager` (approves all orders).
/// A future version will support Python callbacks for custom risk logic.
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
        _state: &Self::State,
        cancels: impl IntoIterator<Item = OrderRequestCancel>,
        opens: impl IntoIterator<Item = OrderRequestOpen>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<OrderRequestCancel>>,
        impl IntoIterator<Item = RiskApproved<OrderRequestOpen>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestCancel>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestOpen>>,
    ) {
        // Default: approve everything
        let approved_cancels: Vec<_> = cancels.into_iter().map(RiskApproved::new).collect();
        let approved_opens: Vec<_> = opens.into_iter().map(RiskApproved::new).collect();
        let refused_cancels: Vec<RiskRefused<OrderRequestCancel>> = vec![];
        let refused_opens: Vec<RiskRefused<OrderRequestOpen>> = vec![];
        (approved_cancels, approved_opens, refused_cancels, refused_opens)
    }
}

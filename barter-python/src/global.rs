use crate::account::PyTradeFill;
use barter::engine::Processor;
use barter_data::event::MarketEvent;
use barter_execution::{AccountEvent, AccountEventKind};
use barter_instrument::{
    asset::AssetIndex,
    exchange::ExchangeIndex,
    instrument::InstrumentIndex,
};
use pyo3::prelude::*;

/// Custom global data that can hold Python callback references.
///
/// Replaces `DefaultGlobalData` to enable `on_fill` event callbacks.
/// The engine calls `global.process(&account_event)` on every account event,
/// so we can intercept trade fills here.
#[derive(Debug)]
pub struct PyGlobalData {
    on_fill: Option<PyObject>,
}

impl Clone for PyGlobalData {
    fn clone(&self) -> Self {
        Self {
            on_fill: self.on_fill.as_ref().map(|obj| {
                Python::with_gil(|py| obj.clone_ref(py))
            }),
        }
    }
}

impl PyGlobalData {
    pub fn new(on_fill: Option<PyObject>) -> Self {
        Self { on_fill }
    }
}

impl Default for PyGlobalData {
    fn default() -> Self {
        Self { on_fill: None }
    }
}

// Concrete impl for the types used by the Python engine
impl Processor<&AccountEvent<ExchangeIndex, AssetIndex, InstrumentIndex>> for PyGlobalData {
    type Audit = ();

    fn process(
        &mut self,
        event: &AccountEvent<ExchangeIndex, AssetIndex, InstrumentIndex>,
    ) -> Self::Audit {
        let Some(ref cb) = self.on_fill else {
            return;
        };

        let AccountEventKind::Trade(trade) = &event.kind else {
            return;
        };

        let fill = PyTradeFill::from_trade(trade);
        Python::with_gil(|py| {
            if let Err(e) = cb.call1(py, (fill,)) {
                eprintln!("Python on_fill callback error: {e}");
            }
        });
    }
}

impl<InstrumentKey, Kind> Processor<&MarketEvent<InstrumentKey, Kind>> for PyGlobalData {
    type Audit = ();
    fn process(&mut self, _: &MarketEvent<InstrumentKey, Kind>) -> Self::Audit {}
}

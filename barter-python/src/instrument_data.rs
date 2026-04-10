use crate::account::PyPositionExited;
use barter::engine::{
    Processor,
    state::{
        instrument::data::{DefaultInstrumentMarketData, InstrumentDataState},
        order::in_flight_recorder::InFlightRequestRecorder,
        position::PositionManager,
    },
};
use barter_data::event::{DataKind, MarketEvent};
use barter_execution::{
    AccountEvent, AccountEventKind,
    order::{
        id::StrategyId,
        request::{OrderRequestCancel, OrderRequestOpen},
    },
};
use barter_instrument::{
    exchange::ExchangeIndex,
    instrument::InstrumentIndex,
};
use fnv::FnvHashMap;
use pyo3::prelude::*;
use rust_decimal::Decimal;

/// Per-strategy instrument data: tracks positions per strategy name.
#[derive(Debug, Clone)]
struct StrategyData {
    position: PositionManager,
}

impl Default for StrategyData {
    fn default() -> Self {
        Self {
            position: PositionManager::default(),
        }
    }
}

/// Custom instrument data for the Python engine.
///
/// Wraps `DefaultInstrumentMarketData` for market prices and adds per-strategy
/// position tracking. Also holds an optional `on_position_closed` Python callback.
#[derive(Debug)]
pub struct PyInstrumentData {
    pub market_data: DefaultInstrumentMarketData,
    pub strategies: FnvHashMap<StrategyId, StrategyData>,
    on_position_closed: Option<PyObject>,
}

impl Clone for PyInstrumentData {
    fn clone(&self) -> Self {
        Self {
            market_data: self.market_data.clone(),
            strategies: self.strategies.clone(),
            on_position_closed: self.on_position_closed.as_ref().map(|obj| {
                Python::with_gil(|py| obj.clone_ref(py))
            }),
        }
    }
}

impl PyInstrumentData {
    pub fn new(strategy_ids: &[StrategyId], on_position_closed: Option<PyObject>) -> Self {
        let mut strategies = FnvHashMap::default();
        for id in strategy_ids {
            strategies.insert(id.clone(), StrategyData::default());
        }
        Self {
            market_data: DefaultInstrumentMarketData::default(),
            strategies,
            on_position_closed,
        }
    }

    /// Check if a strategy has an open position on this instrument.
    pub fn strategy_position(
        &self,
        strategy_id: &StrategyId,
    ) -> Option<&PositionManager> {
        self.strategies.get(strategy_id).map(|d| &d.position)
    }
}

// --- Required trait impls ---

impl InstrumentDataState for PyInstrumentData {
    type MarketEventKind = DataKind;

    fn price(&self) -> Option<Decimal> {
        self.market_data.price()
    }
}

impl<InstrumentKey> Processor<&MarketEvent<InstrumentKey, DataKind>> for PyInstrumentData {
    type Audit = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey, DataKind>) -> Self::Audit {
        self.market_data.process(event)
    }
}

impl Processor<&AccountEvent> for PyInstrumentData {
    type Audit = ();

    fn process(&mut self, event: &AccountEvent) -> Self::Audit {
        let AccountEventKind::Trade(trade) = &event.kind else {
            return;
        };

        // Route trade to the correct strategy by StrategyId
        if let Some(strategy_data) = self.strategies.get_mut(&trade.strategy) {
            let closed = strategy_data.position.update_from_trade(trade);

            // Fire on_position_closed callback if position was fully exited
            if let Some(ref pos_exited) = closed {
                if let Some(ref cb) = self.on_position_closed {
                    let py_pos = PyPositionExited::from_position_exited(pos_exited);
                    Python::with_gil(|py| {
                        if let Err(e) = cb.call1(py, (py_pos,)) {
                            eprintln!("Python on_position_closed callback error: {e}");
                        }
                    });
                }
            }
        }
    }
}

impl InFlightRequestRecorder for PyInstrumentData {
    fn record_in_flight_cancel(
        &mut self,
        _: &OrderRequestCancel<ExchangeIndex, InstrumentIndex>,
    ) {
    }
    fn record_in_flight_open(
        &mut self,
        _: &OrderRequestOpen<ExchangeIndex, InstrumentIndex>,
    ) {
    }
}

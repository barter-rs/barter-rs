use crate::decimal::decimal_to_f64;
use barter_execution::{
    AccountEvent, AccountEventKind,
    trade::Trade,
};
use barter_instrument::{
    Side,
    asset::{AssetIndex, QuoteAsset},
    exchange::ExchangeIndex,
    instrument::InstrumentIndex,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

// ---------------------------------------------------------------------------
// PyTrade — a fill event
// ---------------------------------------------------------------------------

#[pyclass(name = "TradeFill", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyTradeFill {
    pub trade_id: String,
    pub order_id: String,
    pub instrument_index: usize,
    pub strategy_id: String,
    pub time_exchange: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub fees: f64,
}

impl PyTradeFill {
    pub fn from_trade(trade: &Trade<QuoteAsset, InstrumentIndex>) -> Self {
        Self {
            trade_id: trade.id.0.to_string(),
            order_id: trade.order_id.0.to_string(),
            instrument_index: trade.instrument.0,
            strategy_id: trade.strategy.0.to_string(),
            time_exchange: trade.time_exchange.to_rfc3339(),
            side: match trade.side {
                Side::Buy => "buy".to_string(),
                Side::Sell => "sell".to_string(),
            },
            price: decimal_to_f64(&trade.price),
            quantity: decimal_to_f64(&trade.quantity),
            fees: decimal_to_f64(&trade.fees.fees),
        }
    }
}

#[pymethods]
impl PyTradeFill {
    #[getter]
    fn trade_id(&self) -> &str { &self.trade_id }
    #[getter]
    fn order_id(&self) -> &str { &self.order_id }
    #[getter]
    fn instrument_index(&self) -> usize { self.instrument_index }
    #[getter]
    fn strategy_id(&self) -> &str { &self.strategy_id }
    #[getter]
    fn time_exchange(&self) -> &str { &self.time_exchange }
    #[getter]
    fn side(&self) -> &str { &self.side }
    #[getter]
    fn price(&self) -> f64 { self.price }
    #[getter]
    fn quantity(&self) -> f64 { self.quantity }
    #[getter]
    fn fees(&self) -> f64 { self.fees }

    /// Notional value of the fill (price * |quantity|).
    #[getter]
    fn notional(&self) -> f64 {
        self.price * self.quantity.abs()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let d = PyDict::new(py);
        d.set_item("trade_id", &self.trade_id)?;
        d.set_item("order_id", &self.order_id)?;
        d.set_item("instrument_index", self.instrument_index)?;
        d.set_item("strategy_id", &self.strategy_id)?;
        d.set_item("time_exchange", &self.time_exchange)?;
        d.set_item("side", &self.side)?;
        d.set_item("price", self.price)?;
        d.set_item("quantity", self.quantity)?;
        d.set_item("fees", self.fees)?;
        d.set_item("notional", self.notional())?;
        Ok(d.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "TradeFill(instrument={}, {} {} @ {:.2}, qty={}, fees={:.6})",
            self.instrument_index, self.side, self.strategy_id, self.price, self.quantity, self.fees,
        )
    }
}

// ---------------------------------------------------------------------------
// PyAccountEvent — wraps the Rust AccountEvent enum
// ---------------------------------------------------------------------------

#[pyclass(name = "AccountEvent", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyAccountEvent {
    pub exchange_index: usize,
    pub kind: String,
    pub trade: Option<PyTradeFill>,
}

impl PyAccountEvent {
    pub fn from_account_event(event: &AccountEvent) -> Self {
        let (kind, trade) = match &event.kind {
            AccountEventKind::Trade(t) => ("trade".to_string(), Some(PyTradeFill::from_trade(t))),
            AccountEventKind::BalanceSnapshot(_) => ("balance_snapshot".to_string(), None),
            AccountEventKind::OrderSnapshot(_) => ("order_snapshot".to_string(), None),
            AccountEventKind::OrderCancelled(_) => ("order_cancelled".to_string(), None),
            AccountEventKind::Snapshot(_) => ("account_snapshot".to_string(), None),
        };

        Self {
            exchange_index: event.exchange.0,
            kind,
            trade,
        }
    }
}

#[pymethods]
impl PyAccountEvent {
    #[getter]
    fn exchange_index(&self) -> usize { self.exchange_index }
    #[getter]
    fn kind(&self) -> &str { &self.kind }
    #[getter]
    fn trade(&self) -> Option<PyTradeFill> { self.trade.clone() }

    fn __repr__(&self) -> String {
        format!(
            "AccountEvent(exchange={}, kind='{}')",
            self.exchange_index, self.kind,
        )
    }
}

// ---------------------------------------------------------------------------
// PyPositionExited — a closed position
// ---------------------------------------------------------------------------

#[pyclass(name = "PositionExited", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyPositionExited {
    pub instrument_index: usize,
    pub side: String,
    pub price_entry_average: f64,
    pub quantity_abs_max: f64,
    pub pnl_realised: f64,
    pub time_enter: String,
    pub time_exit: String,
    pub trade_count: usize,
}

impl PyPositionExited {
    pub fn from_position_exited(
        pos: &barter::engine::state::position::PositionExited<
            barter_instrument::asset::QuoteAsset,
            barter_instrument::instrument::InstrumentIndex,
        >,
    ) -> Self {
        Self {
            instrument_index: pos.instrument.0,
            side: match pos.side {
                Side::Buy => "buy".to_string(),
                Side::Sell => "sell".to_string(),
            },
            price_entry_average: decimal_to_f64(&pos.price_entry_average),
            quantity_abs_max: decimal_to_f64(&pos.quantity_abs_max),
            pnl_realised: decimal_to_f64(&pos.pnl_realised),
            time_enter: pos.time_enter.to_rfc3339(),
            time_exit: pos.time_exit.to_rfc3339(),
            trade_count: pos.trades.len(),
        }
    }
}

#[pymethods]
impl PyPositionExited {
    #[getter]
    fn instrument_index(&self) -> usize { self.instrument_index }
    #[getter]
    fn side(&self) -> &str { &self.side }
    #[getter]
    fn price_entry_average(&self) -> f64 { self.price_entry_average }
    #[getter]
    fn quantity_abs_max(&self) -> f64 { self.quantity_abs_max }
    #[getter]
    fn pnl_realised(&self) -> f64 { self.pnl_realised }
    #[getter]
    fn time_enter(&self) -> &str { &self.time_enter }
    #[getter]
    fn time_exit(&self) -> &str { &self.time_exit }
    #[getter]
    fn trade_count(&self) -> usize { self.trade_count }

    fn __repr__(&self) -> String {
        format!(
            "PositionExited(instrument={}, side='{}', pnl={:.2}, qty={}, trades={})",
            self.instrument_index, self.side, self.pnl_realised, self.quantity_abs_max, self.trade_count,
        )
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyTradeFill>()?;
    parent.add_class::<PyAccountEvent>()?;
    parent.add_class::<PyPositionExited>()?;
    Ok(())
}

use crate::decimal::{decimal_to_f64, py_to_decimal};
use barter_execution::order::{
    OrderKind, TimeInForce,
    id::{ClientOrderId, OrderId, StrategyId},
    request::{OrderRequestCancel, OrderRequestOpen, RequestCancel, RequestOpen},
};
use barter_instrument::{
    Side,
    exchange::ExchangeIndex,
    instrument::InstrumentIndex,
};
use barter_execution::order::OrderKey;
use pyo3::prelude::*;
use rust_decimal::Decimal;

// ---------------------------------------------------------------------------
// PyOrderRequestOpen — Python-constructible order open request
// ---------------------------------------------------------------------------

#[pyclass(name = "OrderRequestOpen", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyOrderRequestOpen {
    pub exchange_index: usize,
    pub instrument_index: usize,
    pub strategy_id: String,
    pub side: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub order_kind: String,
    pub time_in_force: String,
}

#[pymethods]
impl PyOrderRequestOpen {
    #[new]
    #[pyo3(signature = (exchange_index, instrument_index, side, price, quantity, order_kind="market", time_in_force="ioc", strategy_id="default"))]
    fn new(
        exchange_index: usize,
        instrument_index: usize,
        side: &str,
        price: &Bound<'_, PyAny>,
        quantity: &Bound<'_, PyAny>,
        order_kind: &str,
        time_in_force: &str,
        strategy_id: &str,
    ) -> PyResult<Self> {
        // Validate side
        match side.to_lowercase().as_str() {
            "buy" | "b" | "sell" | "s" => {}
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid side: '{other}', use 'buy' or 'sell'"
                )));
            }
        }
        // Validate order_kind
        match order_kind.to_lowercase().as_str() {
            "market" | "limit" => {}
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid order_kind: '{other}', use 'market' or 'limit'"
                )));
            }
        }

        Ok(Self {
            exchange_index,
            instrument_index,
            strategy_id: strategy_id.to_string(),
            side: side.to_lowercase(),
            price: py_to_decimal(price)?,
            quantity: py_to_decimal(quantity)?,
            order_kind: order_kind.to_lowercase(),
            time_in_force: time_in_force.to_lowercase(),
        })
    }

    #[getter]
    fn exchange_index(&self) -> usize {
        self.exchange_index
    }

    #[getter]
    fn instrument_index(&self) -> usize {
        self.instrument_index
    }

    #[getter]
    fn strategy_id(&self) -> &str {
        &self.strategy_id
    }

    #[getter]
    fn side(&self) -> &str {
        &self.side
    }

    #[getter]
    fn price(&self) -> f64 {
        decimal_to_f64(&self.price)
    }

    #[getter]
    fn quantity(&self) -> f64 {
        decimal_to_f64(&self.quantity)
    }

    #[getter]
    fn order_kind(&self) -> &str {
        &self.order_kind
    }

    #[getter]
    fn time_in_force(&self) -> &str {
        &self.time_in_force
    }

    fn __repr__(&self) -> String {
        format!(
            "OrderRequestOpen(exchange={}, instrument={}, side='{}', price={}, qty={}, kind='{}', strategy='{}')",
            self.exchange_index, self.instrument_index, self.side,
            self.price, self.quantity, self.order_kind, self.strategy_id,
        )
    }
}

impl PyOrderRequestOpen {
    /// Convert to the Rust OrderRequestOpen type used by the engine.
    pub fn to_rust(&self) -> OrderRequestOpen<ExchangeIndex, InstrumentIndex> {
        let side = match self.side.as_str() {
            "buy" | "b" => Side::Buy,
            _ => Side::Sell,
        };
        let kind = match self.order_kind.as_str() {
            "limit" => OrderKind::Limit,
            _ => OrderKind::Market,
        };
        let tif = match self.time_in_force.as_str() {
            "gtc" => TimeInForce::GoodUntilCancelled { post_only: false },
            "gtd" => TimeInForce::GoodUntilEndOfDay,
            "fok" => TimeInForce::FillOrKill,
            _ => TimeInForce::ImmediateOrCancel,
        };

        OrderRequestOpen {
            key: OrderKey {
                exchange: ExchangeIndex(self.exchange_index),
                instrument: InstrumentIndex(self.instrument_index),
                strategy: StrategyId::new(&self.strategy_id),
                cid: ClientOrderId::random(),
            },
            state: RequestOpen {
                side,
                price: self.price,
                quantity: self.quantity,
                kind,
                time_in_force: tif,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// PyOrderRequestCancel — Python-constructible order cancel request
// ---------------------------------------------------------------------------

#[pyclass(name = "OrderRequestCancel", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyOrderRequestCancel {
    pub exchange_index: usize,
    pub instrument_index: usize,
    pub strategy_id: String,
    pub order_id: Option<String>,
}

#[pymethods]
impl PyOrderRequestCancel {
    #[new]
    #[pyo3(signature = (exchange_index, instrument_index, strategy_id="default", order_id=None))]
    fn new(
        exchange_index: usize,
        instrument_index: usize,
        strategy_id: &str,
        order_id: Option<String>,
    ) -> Self {
        Self {
            exchange_index,
            instrument_index,
            strategy_id: strategy_id.to_string(),
            order_id,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "OrderRequestCancel(exchange={}, instrument={}, order_id={:?}, strategy='{}')",
            self.exchange_index, self.instrument_index, self.order_id, self.strategy_id,
        )
    }
}

impl PyOrderRequestCancel {
    /// Convert to the Rust OrderRequestCancel type used by the engine.
    pub fn to_rust(&self) -> OrderRequestCancel<ExchangeIndex, InstrumentIndex> {
        OrderRequestCancel {
            key: OrderKey {
                exchange: ExchangeIndex(self.exchange_index),
                instrument: InstrumentIndex(self.instrument_index),
                strategy: StrategyId::new(&self.strategy_id),
                cid: ClientOrderId::random(),
            },
            state: RequestCancel {
                id: self.order_id.as_ref().map(|id| OrderId::from(smol_str::SmolStr::from(id.as_str()))),
            },
        }
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyOrderRequestOpen>()?;
    parent.add_class::<PyOrderRequestCancel>()?;
    Ok(())
}

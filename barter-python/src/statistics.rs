use crate::instrument::PyIndexedInstruments;
use barter::{
    engine::state::{
        EngineState,
        global::DefaultGlobalData,
        instrument::data::DefaultInstrumentMarketData,
        position::PositionExited,
        trading::TradingState,
    },
    statistic::{
        summary::{TradingSummary, TradingSummaryGenerator},
        time::{Annual365, Daily},
    },
};
use barter_execution::balance::{AssetBalance, Balance};
use barter_execution::trade::AssetFees;
use barter_instrument::{
    Side,
    asset::{AssetIndex, QuoteAsset},
    instrument::InstrumentIndex,
};
use barter_integration::collection::snapshot::Snapshot;
use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rust_decimal::Decimal;

// ---------------------------------------------------------------------------
// TradingSummary wrapper (generic over interval — we monomorphize)
// ---------------------------------------------------------------------------

#[pyclass(name = "TradingSummary", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyTradingSummary {
    /// We store the summary as a JSON value so we can support any interval.
    summary_json: String,
    time_start: String,
    time_end: String,
    duration_secs: f64,
}

impl PyTradingSummary {
    pub fn from_daily(summary: &TradingSummary<Daily>) -> Self {
        Self {
            summary_json: serde_json::to_string_pretty(summary).unwrap_or_default(),
            time_start: summary.time_engine_start.to_rfc3339(),
            time_end: summary.time_engine_end.to_rfc3339(),
            duration_secs: summary.trading_duration().num_seconds() as f64,
        }
    }

    pub fn from_annual365(summary: &TradingSummary<Annual365>) -> Self {
        Self {
            summary_json: serde_json::to_string_pretty(summary).unwrap_or_default(),
            time_start: summary.time_engine_start.to_rfc3339(),
            time_end: summary.time_engine_end.to_rfc3339(),
            duration_secs: summary.trading_duration().num_seconds() as f64,
        }
    }
}

#[pymethods]
impl PyTradingSummary {
    #[getter]
    fn time_start(&self) -> &str {
        &self.time_start
    }

    #[getter]
    fn time_end(&self) -> &str {
        &self.time_end
    }

    #[getter]
    fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    /// Get the full summary as a JSON string.
    fn to_json(&self) -> &str {
        &self.summary_json
    }

    /// Get the full summary as a Python dict.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let json_mod = py.import("json")?;
        let result = json_mod.call_method1("loads", (&self.summary_json,))?;
        result.downcast_into::<PyDict>().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("failed to parse summary: {e}"))
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "TradingSummary(start='{}', end='{}', duration={:.0}s)",
            self.time_start, self.time_end, self.duration_secs,
        )
    }
}

// ---------------------------------------------------------------------------
// TradingSummaryGenerator wrapper
// ---------------------------------------------------------------------------

#[pyclass(name = "TradingSummaryGenerator", module = "barter")]
pub struct PyTradingSummaryGenerator {
    inner: TradingSummaryGenerator,
}

#[pymethods]
impl PyTradingSummaryGenerator {
    /// Create a new generator from IndexedInstruments.
    ///
    /// Args:
    ///     instruments: IndexedInstruments
    ///     risk_free_return: float (e.g. 0.05 for 5%)
    #[new]
    fn new(instruments: &PyIndexedInstruments, risk_free_return: f64) -> PyResult<Self> {
        let rfr = Decimal::try_from(risk_free_return).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("invalid risk_free_return: {e}"))
        })?;

        let time_now = Utc::now();

        // Build a temporary EngineState to initialise the generator
        let state = EngineState::builder(
            &instruments.inner,
            DefaultGlobalData::default(),
            |_| DefaultInstrumentMarketData::default(),
        )
        .time_engine_start(time_now)
        .trading_state(TradingState::Enabled)
        .build();

        let generator = TradingSummaryGenerator::init(
            rfr,
            time_now,
            time_now,
            &state.instruments,
            &state.assets,
        );

        Ok(Self { inner: generator })
    }

    /// Update with a balance change.
    ///
    /// Args:
    ///     asset_index: int — the asset index
    ///     total: float — new total balance
    ///     free: float — new free balance
    ///     time_exchange: str — ISO 8601 timestamp
    fn update_balance(
        &mut self,
        asset_index: usize,
        total: f64,
        free: f64,
        time_exchange: &str,
    ) -> PyResult<()> {
        let time: DateTime<Utc> = time_exchange
            .parse()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("bad timestamp: {e}")))?;

        let total_dec = Decimal::try_from(total)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let free_dec = Decimal::try_from(free)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let balance = Snapshot::new(AssetBalance {
            asset: AssetIndex(asset_index),
            balance: Balance::new(total_dec, free_dec),
            time_exchange: time,
        });

        self.inner.update_from_balance(balance.as_ref());
        Ok(())
    }

    /// Update with a closed position.
    ///
    /// Args:
    ///     instrument_index: int
    ///     side: str — "buy" or "sell"
    ///     pnl_realised: float — realised PnL in quote currency
    ///     quantity: float — max absolute quantity
    ///     time_enter: str — ISO 8601
    ///     time_exit: str — ISO 8601
    fn update_position(
        &mut self,
        instrument_index: usize,
        side: &str,
        pnl_realised: f64,
        quantity: f64,
        time_enter: &str,
        time_exit: &str,
    ) -> PyResult<()> {
        let side = match side.to_lowercase().as_str() {
            "buy" | "b" => Side::Buy,
            "sell" | "s" => Side::Sell,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "invalid side: {other}"
                )))
            }
        };

        let t_enter: DateTime<Utc> = time_enter
            .parse()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("bad timestamp: {e}")))?;
        let t_exit: DateTime<Utc> = time_exit
            .parse()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("bad timestamp: {e}")))?;

        let pnl = Decimal::try_from(pnl_realised)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let qty = Decimal::try_from(quantity)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let position = PositionExited {
            instrument: InstrumentIndex(instrument_index),
            side,
            price_entry_average: Decimal::ONE,
            quantity_abs_max: qty,
            pnl_realised: pnl,
            fees_enter: AssetFees::<QuoteAsset>::default(),
            fees_exit: AssetFees::<QuoteAsset>::default(),
            time_enter: t_enter,
            time_exit: t_exit,
            trades: vec![],
        };

        self.inner.update_from_position(&position);
        Ok(())
    }

    /// Generate a TradingSummary.
    ///
    /// Args:
    ///     interval: str — "daily", "annual365", or "annual252"
    #[pyo3(signature = (interval="daily"))]
    fn generate(&self, interval: &str) -> PyResult<PyTradingSummary> {
        match interval {
            "daily" => {
                let summary = self.inner.clone().generate(Daily);
                Ok(PyTradingSummary::from_daily(&summary))
            }
            "annual365" | "annual_365" | "crypto" => {
                let summary = self.inner.clone().generate(Annual365);
                Ok(PyTradingSummary::from_annual365(&summary))
            }
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown interval: '{other}'. Use 'daily' or 'annual365'."
            ))),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TradingSummaryGenerator(instruments={}, assets={})",
            self.inner.instruments.len(),
            self.inner.assets.len(),
        )
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyTradingSummary>()?;
    parent.add_class::<PyTradingSummaryGenerator>()?;
    Ok(())
}

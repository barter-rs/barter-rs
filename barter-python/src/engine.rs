use crate::global::PyGlobalData;
use crate::instrument_data::PyInstrumentData;
use crate::risk::PyRiskManager;
use crate::statistics::PyTradingSummary;
use crate::strategy::PyStrategy;
use barter::{
    engine::{
        clock::HistoricalClock,
        state::{
            EngineState,
            trading::TradingState,
        },
    },
    execution::builder::{ExecutionBuild, ExecutionBuilder},
    statistic::time::Daily,
    system::{
        builder::{AuditMode, EngineFeedMode, SystemBuild},
        config::{ExecutionConfig, SystemConfig},
    },
};
use barter_data::{
    event::DataKind,
    streams::consumer::MarketStreamResult,
    streams::reconnect::stream::ReconnectingStream,
};
use barter_execution::order::id::StrategyId;
use barter_instrument::{index::IndexedInstruments, instrument::InstrumentIndex};
use futures::stream;
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use rust_decimal::Decimal;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// run_backtest — high-level Python function
// ---------------------------------------------------------------------------

/// Run a backtest with historical market data.
///
/// Args:
///     config_json: str — JSON string matching SystemConfig format
///     market_data_json: str — JSON array of MarketStreamResult events
///     risk_free_return: float — risk-free rate (e.g. 0.05)
///     strategy: callable — single strategy: fn(state) -> List[OrderRequestOpen]
///     strategies: dict — multi-strategy: {"name": fn(state) -> List[OrderRequestOpen], ...}
///     risk: callable — fn(state, opens) -> List[OrderRequestOpen]
///     on_fill: callable — fn(trade: TradeFill) called on every trade fill
///     on_position_closed: callable — fn(pos: PositionExited) called when a position closes
///
/// Returns:
///     TradingSummary
#[pyfunction]
#[pyo3(signature = (config_json, market_data_json, risk_free_return=0.05, strategy=None, strategies=None, risk=None, on_fill=None, on_position_closed=None))]
pub fn run_backtest<'py>(
    py: Python<'py>,
    config_json: String,
    market_data_json: String,
    risk_free_return: f64,
    strategy: Option<PyObject>,
    strategies: Option<HashMap<String, PyObject>>,
    risk: Option<PyObject>,
    on_fill: Option<PyObject>,
    on_position_closed: Option<PyObject>,
) -> PyResult<Bound<'py, PyAny>> {
    // Determine strategy IDs for instrument data
    let strategy_ids: Vec<StrategyId> = if let Some(ref strats) = strategies {
        strats.keys().map(|k| StrategyId::new(k)).collect()
    } else {
        vec![StrategyId::new("default")]
    };

    // Build the combined strategy callback
    let py_strategy = if let Some(strats) = strategies {
        // Multi-strategy: each entry is (name, callback)
        let combined: Vec<(StrategyId, PyObject)> = strats
            .into_iter()
            .map(|(name, cb)| (StrategyId::new(&name), cb))
            .collect();
        PyStrategy::new_multi(combined)
    } else {
        PyStrategy::new(strategy)
    };

    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        // Parse config
        let config: SystemConfig = serde_json::from_str(&config_json)
            .map_err(|e| PyRuntimeError::new_err(format!("invalid config JSON: {e}")))?;

        let instruments = IndexedInstruments::new(config.instruments);

        // Parse historical market data
        let events: Vec<MarketStreamResult<InstrumentIndex, DataKind>> =
            serde_json::from_str(&market_data_json)
                .map_err(|e| PyRuntimeError::new_err(format!("invalid market data JSON: {e}")))?;

        // Find first event time for the historical clock
        let time_first = events
            .iter()
            .find_map(|result| match result {
                MarketStreamResult::Item(Ok(event)) => Some(event.time_exchange),
                _ => None,
            })
            .ok_or_else(|| PyRuntimeError::new_err("market data contains no valid events"))?;

        let clock = HistoricalClock::new(time_first);

        let market_stream = stream::iter(events)
            .with_error_handler(|error| {
                eprintln!("backtest stream error: {error:?}");
            });

        // Build execution infrastructure
        let execution_build = config
            .executions
            .into_iter()
            .try_fold(
                ExecutionBuilder::new(&instruments),
                |builder, exec_config| match exec_config {
                    ExecutionConfig::Mock(mock_config) => builder.add_mock(mock_config, clock.clone()),
                },
            )
            .map_err(|e| PyRuntimeError::new_err(format!("execution build failed: {e:?}")))?
            .build();

        let ExecutionBuild {
            execution_tx_map,
            account_channel,
            futures,
        } = execution_build;

        // Build EngineState with PyGlobalData and PyInstrumentData
        let global_data = PyGlobalData::new(on_fill);

        // Clone the PyObject callback for each instrument's data
        let on_pos_closed_for_instruments = on_position_closed.map(|obj| {
            Python::with_gil(|py| obj.clone_ref(py))
        });

        let engine_state = EngineState::builder(
            &instruments,
            global_data,
            |_| {
                let cb = on_pos_closed_for_instruments.as_ref().map(|obj| {
                    Python::with_gil(|py| obj.clone_ref(py))
                });
                PyInstrumentData::new(&strategy_ids, cb)
            },
        )
        .time_engine_start(time_first)
        .trading_state(TradingState::Enabled)
        .build();

        // Build Engine
        let py_risk = PyRiskManager::new(risk);

        let engine = barter::engine::Engine::new(
            clock,
            engine_state,
            execution_tx_map,
            py_strategy,
            py_risk,
        );

        // Build and run system
        let system = SystemBuild::new(
            engine,
            EngineFeedMode::Stream,
            AuditMode::Disabled,
            market_stream,
            account_channel,
            futures,
        )
        .init()
        .await
        .map_err(|e| PyRuntimeError::new_err(format!("system init failed: {e:?}")))?;

        let (engine, _) = system
            .shutdown_after_backtest()
            .await
            .map_err(|e| PyRuntimeError::new_err(format!("backtest failed: {e:?}")))?;

        let rfr = Decimal::try_from(risk_free_return)
            .map_err(|e| PyRuntimeError::new_err(format!("bad risk_free_return: {e}")))?;

        let summary = engine
            .trading_summary_generator(rfr)
            .generate(Daily);

        PyTradingSummary::from_daily(&summary)
    })
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_function(wrap_pyfunction!(run_backtest, parent)?)?;
    Ok(())
}

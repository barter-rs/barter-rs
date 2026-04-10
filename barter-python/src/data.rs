use crate::decimal::decimal_to_f64;
use crate::execution::PyPublicTrade;
use crate::instrument::PyExchangeId;
use barter_data::{
    event::{DataKind, MarketEvent},
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        coinbase::Coinbase,
        okx::Okx,
    },
    streams::{Streams, reconnect::{Event, stream::ReconnectingStream}},
    subscription::trade::PublicTrades,
};
use barter_instrument::{
    exchange::ExchangeId,
    instrument::market_data::{MarketDataInstrument, kind::MarketDataInstrumentKind},
};
use futures_util::StreamExt;
use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyStopAsyncIteration};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

// ---------------------------------------------------------------------------
// MarketEvent wrapper
// ---------------------------------------------------------------------------

#[pyclass(name = "MarketEvent", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyMarketEvent {
    pub time_exchange: String,
    pub time_received: String,
    pub exchange: ExchangeId,
    pub instrument: String,
    pub kind_name: String,
    pub trade: Option<PyPublicTrade>,
    pub best_bid_price: Option<f64>,
    pub best_bid_amount: Option<f64>,
    pub best_ask_price: Option<f64>,
    pub best_ask_amount: Option<f64>,
}

impl PyMarketEvent {
    pub fn from_market_event(event: &MarketEvent<MarketDataInstrument, DataKind>) -> Self {
        let (kind_name, trade, best_bid_price, best_bid_amount, best_ask_price, best_ask_amount) =
            match &event.kind {
                DataKind::Trade(t) => (
                    "trade".to_string(),
                    Some(PyPublicTrade {
                        id: t.id.clone(),
                        price: t.price,
                        amount: t.amount,
                        side: t.side,
                    }),
                    None, None, None, None,
                ),
                DataKind::OrderBookL1(book) => (
                    "order_book_l1".to_string(),
                    None,
                    book.best_bid.as_ref().map(|l| decimal_to_f64(&l.price)),
                    book.best_bid.as_ref().map(|l| decimal_to_f64(&l.amount)),
                    book.best_ask.as_ref().map(|l| decimal_to_f64(&l.price)),
                    book.best_ask.as_ref().map(|l| decimal_to_f64(&l.amount)),
                ),
                _ => (
                    "other".to_string(),
                    None, None, None, None, None,
                ),
            };

        Self {
            time_exchange: event.time_exchange.to_rfc3339(),
            time_received: event.time_received.to_rfc3339(),
            exchange: event.exchange,
            instrument: event.instrument.to_string(),
            kind_name,
            trade,
            best_bid_price,
            best_bid_amount,
            best_ask_price,
            best_ask_amount,
        }
    }

    pub fn from_trade_event(event: &MarketEvent<MarketDataInstrument, barter_data::subscription::trade::PublicTrade>) -> Self {
        Self {
            time_exchange: event.time_exchange.to_rfc3339(),
            time_received: event.time_received.to_rfc3339(),
            exchange: event.exchange,
            instrument: event.instrument.to_string(),
            kind_name: "trade".to_string(),
            trade: Some(PyPublicTrade {
                id: event.kind.id.clone(),
                price: event.kind.price,
                amount: event.kind.amount,
                side: event.kind.side,
            }),
            best_bid_price: None,
            best_bid_amount: None,
            best_ask_price: None,
            best_ask_amount: None,
        }
    }
}

#[pymethods]
impl PyMarketEvent {
    #[getter]
    fn time_exchange(&self) -> &str {
        &self.time_exchange
    }

    #[getter]
    fn time_received(&self) -> &str {
        &self.time_received
    }

    #[getter]
    fn exchange(&self) -> PyExchangeId {
        PyExchangeId(self.exchange)
    }

    #[getter]
    fn instrument(&self) -> &str {
        &self.instrument
    }

    #[getter]
    fn kind(&self) -> &str {
        &self.kind_name
    }

    #[getter]
    fn trade(&self) -> Option<PyPublicTrade> {
        self.trade.clone()
    }

    #[getter]
    fn best_bid_price(&self) -> Option<f64> {
        self.best_bid_price
    }

    #[getter]
    fn best_ask_price(&self) -> Option<f64> {
        self.best_ask_price
    }

    fn __repr__(&self) -> String {
        let exchange_str = serde_json::to_value(&self.exchange)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self.exchange));
        format!(
            "MarketEvent(exchange='{}', instrument='{}', kind='{}')",
            exchange_str,
            self.instrument,
            self.kind_name,
        )
    }
}

// ---------------------------------------------------------------------------
// MarketDataStream — Python async iterator over market events
// ---------------------------------------------------------------------------

#[pyclass(name = "MarketDataStream", module = "barter")]
pub struct PyMarketDataStream {
    rx: Arc<Mutex<mpsc::UnboundedReceiver<PyMarketEvent>>>,
}

#[pymethods]
impl PyMarketDataStream {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        let rx = self.rx.clone();
        let future = pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut guard = rx.lock().await;
            match guard.recv().await {
                Some(event) => Ok(event),
                None => Err(PyStopAsyncIteration::new_err("")),
            }
        })?;
        Ok(Some(future))
    }
}

// ---------------------------------------------------------------------------
// Subscription spec for the builder
// ---------------------------------------------------------------------------

#[pyclass(name = "Subscription", module = "barter")]
#[derive(Debug, Clone)]
pub struct PySubscription {
    pub exchange: String,
    pub base: String,
    pub quote_asset: String,
    pub instrument_kind: String,
    pub data_kind: String,
}

#[pymethods]
impl PySubscription {
    #[new]
    #[pyo3(signature = (exchange, base, quote, instrument_kind="spot", data_kind="trades"))]
    fn new(
        exchange: String,
        base: String,
        quote: String,
        instrument_kind: &str,
        data_kind: &str,
    ) -> Self {
        Self {
            exchange,
            base,
            quote_asset: quote,
            instrument_kind: instrument_kind.to_string(),
            data_kind: data_kind.to_string(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Subscription(exchange='{}', base='{}', quote='{}', kind='{}', data='{}')",
            self.exchange, self.base, self.quote_asset, self.instrument_kind, self.data_kind,
        )
    }
}

// ---------------------------------------------------------------------------
// build_market_stream — top-level async function
// ---------------------------------------------------------------------------

/// Build and start a market data stream from subscription specs.
///
/// This function blocks briefly to initialise the WebSocket connections on the
/// Tokio runtime, then returns a `MarketDataStream` async iterator.
#[pyfunction]
pub fn build_market_stream(
    py: Python<'_>,
    subscriptions: Vec<PySubscription>,
) -> PyResult<PyMarketDataStream> {
    let mut trade_subs: Vec<(String, String, String, String)> = Vec::new();

    for sub in &subscriptions {
        match sub.data_kind.as_str() {
            "trades" | "public_trades" => {
                trade_subs.push((
                    sub.exchange.clone(),
                    sub.base.clone(),
                    sub.quote_asset.clone(),
                    sub.instrument_kind.clone(),
                ));
            }
            other => {
                return Err(PyRuntimeError::new_err(format!(
                    "unsupported data_kind: '{other}'. Use 'trades'."
                )));
            }
        }
    }

    // Release the GIL while doing blocking I/O on the Tokio runtime
    let result = py.allow_threads(|| {
        crate::runtime::get_runtime().block_on(async {
            let mut builder = Streams::<PublicTrades>::builder();

            // BinanceSpot
            let subs: Vec<_> = trade_subs
                .iter()
                .filter(|(ex, _, _, _)| ex == "binance_spot")
                .map(|(_, base, quote, kind)| {
                    (BinanceSpot::default(), base.as_str(), quote.as_str(), parse_instrument_kind(kind), PublicTrades)
                })
                .collect();
            if !subs.is_empty() {
                builder = builder.subscribe(subs);
            }

            // BinanceFuturesUsd
            let subs: Vec<_> = trade_subs
                .iter()
                .filter(|(ex, _, _, _)| ex == "binance_futures_usd")
                .map(|(_, base, quote, kind)| {
                    (BinanceFuturesUsd::default(), base.as_str(), quote.as_str(), parse_instrument_kind(kind), PublicTrades)
                })
                .collect();
            if !subs.is_empty() {
                builder = builder.subscribe(subs);
            }

            // OKX
            let subs: Vec<_> = trade_subs
                .iter()
                .filter(|(ex, _, _, _)| ex == "okx")
                .map(|(_, base, quote, kind)| {
                    (Okx, base.as_str(), quote.as_str(), parse_instrument_kind(kind), PublicTrades)
                })
                .collect();
            if !subs.is_empty() {
                builder = builder.subscribe(subs);
            }

            // Coinbase
            let subs: Vec<_> = trade_subs
                .iter()
                .filter(|(ex, _, _, _)| ex == "coinbase")
                .map(|(_, base, quote, kind)| {
                    (Coinbase, base.as_str(), quote.as_str(), parse_instrument_kind(kind), PublicTrades)
                })
                .collect();
            if !subs.is_empty() {
                builder = builder.subscribe(subs);
            }

            let streams = builder
                .init()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("failed to init streams: {e:?}")))?;

            let (tx, rx) = mpsc::unbounded_channel();

            let mut joined = streams
                .select_all()
                .with_error_handler(|error| {
                    eprintln!("barter MarketStream error: {error:?}");
                });

            tokio::spawn(async move {
                while let Some(event) = joined.next().await {
                    match event {
                        Event::Item(market_event) => {
                            let py_event = PyMarketEvent::from_trade_event(&market_event);
                            if tx.send(py_event).is_err() {
                                break;
                            }
                        }
                        Event::Reconnecting(_) => {}
                    }
                }
            });

            Ok::<_, PyErr>(rx)
        })
    })?;

    Ok(PyMarketDataStream {
        rx: Arc::new(Mutex::new(result)),
    })
}

fn parse_instrument_kind(kind: &str) -> MarketDataInstrumentKind {
    match kind {
        "spot" => MarketDataInstrumentKind::Spot,
        "perpetual" => MarketDataInstrumentKind::Perpetual,
        _ => MarketDataInstrumentKind::Spot,
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyMarketEvent>()?;
    parent.add_class::<PyMarketDataStream>()?;
    parent.add_class::<PySubscription>()?;
    parent.add_function(wrap_pyfunction!(build_market_stream, parent)?)?;
    Ok(())
}

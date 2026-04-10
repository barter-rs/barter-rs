use crate::decimal::decimal_to_f64;
use barter_data::{
    books::{OrderBook, map::OrderBookMap},
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        bybit::{futures::BybitPerpetualsUsd, spot::BybitSpot},
    },
    books::manager::init_multi_order_book_l2_manager,
    subscription::book::OrderBooksL2,
};
use barter_instrument::instrument::market_data::{
    MarketDataInstrument, kind::MarketDataInstrumentKind,
};
use parking_lot::RwLock;
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::{PyDict, PyList};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// PyOrderBook — Python-friendly L2 order book snapshot
// ---------------------------------------------------------------------------

#[pyclass(name = "OrderBook", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyOrderBook {
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
    pub mid_price: Option<f64>,
    pub sequence: u64,
}

impl PyOrderBook {
    pub fn from_book(book: &OrderBook) -> Self {
        Self {
            bids: book
                .bids()
                .levels()
                .iter()
                .map(|l| (decimal_to_f64(&l.price), decimal_to_f64(&l.amount)))
                .collect(),
            asks: book
                .asks()
                .levels()
                .iter()
                .map(|l| (decimal_to_f64(&l.price), decimal_to_f64(&l.amount)))
                .collect(),
            mid_price: book.mid_price().map(|d| decimal_to_f64(&d)),
            sequence: book.sequence(),
        }
    }
}

#[pymethods]
impl PyOrderBook {
    /// Best bid/ask levels as (price, amount) tuples, sorted best-first.
    #[getter]
    fn bids(&self) -> Vec<(f64, f64)> {
        self.bids.clone()
    }

    #[getter]
    fn asks(&self) -> Vec<(f64, f64)> {
        self.asks.clone()
    }

    #[getter]
    fn mid_price(&self) -> Option<f64> {
        self.mid_price
    }

    #[getter]
    fn sequence(&self) -> u64 {
        self.sequence
    }

    #[getter]
    fn best_bid(&self) -> Option<(f64, f64)> {
        self.bids.first().copied()
    }

    #[getter]
    fn best_ask(&self) -> Option<(f64, f64)> {
        self.asks.first().copied()
    }

    #[getter]
    fn spread(&self) -> Option<f64> {
        match (self.bids.first(), self.asks.first()) {
            (Some((bid, _)), Some((ask, _))) => Some(ask - bid),
            _ => None,
        }
    }

    /// Return a depth-limited snapshot as a dict.
    fn snapshot(&self, depth: usize, py: Python<'_>) -> PyResult<PyObject> {
        let d = PyDict::new(py);
        let bids = PyList::new(py, self.bids.iter().take(depth).copied().collect::<Vec<_>>())?;
        let asks = PyList::new(py, self.asks.iter().take(depth).copied().collect::<Vec<_>>())?;
        d.set_item("bids", bids)?;
        d.set_item("asks", asks)?;
        d.set_item("mid_price", self.mid_price)?;
        d.set_item("spread", self.spread())?;
        d.set_item("sequence", self.sequence)?;
        Ok(d.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "OrderBook(bids={}, asks={}, mid={:?}, seq={})",
            self.bids.len(),
            self.asks.len(),
            self.mid_price,
            self.sequence,
        )
    }

    fn __len__(&self) -> usize {
        self.bids.len() + self.asks.len()
    }
}

// ---------------------------------------------------------------------------
// PyOrderBookManager — manages L2 books in the background
// ---------------------------------------------------------------------------

#[pyclass(name = "OrderBookManager", module = "barter")]
pub struct PyOrderBookManager {
    instruments: Vec<(String, MarketDataInstrument)>,
    books: Vec<(MarketDataInstrument, Arc<RwLock<OrderBook>>)>,
}

#[pymethods]
impl PyOrderBookManager {
    /// Get a snapshot of the order book for the given instrument string.
    fn get(&self, instrument: &str) -> Option<PyOrderBook> {
        self.books
            .iter()
            .find(|(key, _)| key.to_string() == instrument)
            .map(|(_, book)| {
                let lock = book.read();
                PyOrderBook::from_book(&lock)
            })
    }

    /// Get a depth-limited snapshot.
    fn get_depth(&self, instrument: &str, depth: usize) -> Option<PyOrderBook> {
        self.books
            .iter()
            .find(|(key, _)| key.to_string() == instrument)
            .map(|(_, book)| {
                let lock = book.read();
                let snapped = lock.snapshot(depth);
                PyOrderBook::from_book(&snapped)
            })
    }

    /// List all instrument names being tracked.
    fn instruments(&self) -> Vec<String> {
        self.instruments.iter().map(|(name, _)| name.clone()).collect()
    }

    fn __repr__(&self) -> String {
        format!("OrderBookManager(instruments={})", self.instruments.len())
    }

    fn __len__(&self) -> usize {
        self.instruments.len()
    }
}

// ---------------------------------------------------------------------------
// build_order_book_manager — top-level function
// ---------------------------------------------------------------------------

/// Build and start an L2 order book manager that maintains local book snapshots.
///
/// Supported exchanges for L2: binance_spot, binance_futures_usd, bybit_spot, bybit_perpetuals_usd.
///
/// Returns an `OrderBookManager` that provides thread-safe read access to the
/// latest book state. The manager runs in the background updating books from
/// WebSocket streams.
#[pyfunction]
pub fn build_order_book_manager(
    py: Python<'_>,
    subscriptions: Vec<crate::data::PySubscription>,
) -> PyResult<PyOrderBookManager> {
    let subs: Vec<(String, String, String, String)> = subscriptions
        .iter()
        .map(|s| (s.exchange.clone(), s.base.clone(), s.quote_asset.clone(), s.instrument_kind.clone()))
        .collect();

    // Helper macro to init an L2 manager for a given exchange type
    macro_rules! init_l2 {
        ($all_books:expr, $names:expr, $exchange_name:expr, $exchange_type:expr, $subs:expr) => {{
            let exchange_subs: Vec<_> = $subs.iter()
                .filter(|(ex, _, _, _)| ex == $exchange_name)
                .map(|(_, base, quote, kind)| {
                    let b: &str = base;
                    let q: &str = quote;
                    ($exchange_type, b.to_owned(), q.to_owned(), parse_instrument_kind(kind), OrderBooksL2)
                })
                .collect();
            if !exchange_subs.is_empty() {
                let manager = init_multi_order_book_l2_manager([exchange_subs])
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("{} L2 init failed: {e:?}", $exchange_name)))?;
                for key in manager.books.keys() {
                    if let Some(book) = manager.books.find(key) {
                        $names.push((key.to_string(), key.clone()));
                        $all_books.push((key.clone(), book));
                    }
                }
                tokio::spawn(async move { manager.run().await });
            }
        }};
    }

    // Release GIL and block on the async init
    py.allow_threads(|| {
        crate::runtime::get_runtime().block_on(async {
            let mut all_books: Vec<(MarketDataInstrument, Arc<RwLock<OrderBook>>)> = Vec::new();
            let mut instrument_names: Vec<(String, MarketDataInstrument)> = Vec::new();

            init_l2!(all_books, instrument_names, "binance_spot", BinanceSpot::default(), subs);
            init_l2!(all_books, instrument_names, "binance_futures_usd", BinanceFuturesUsd::default(), subs);
            init_l2!(all_books, instrument_names, "bybit_spot", BybitSpot::default(), subs);
            init_l2!(all_books, instrument_names, "bybit_perpetuals_usd", BybitPerpetualsUsd::default(), subs);

            if all_books.is_empty() {
                return Err(PyRuntimeError::new_err(
                    "no L2 subscriptions matched a supported exchange (binance_spot, binance_futures_usd, bybit_spot, bybit_perpetuals_usd)"
                ));
            }

            Ok(PyOrderBookManager {
                instruments: instrument_names,
                books: all_books,
            })
        })
    })
}

fn parse_instrument_kind(kind: &str) -> MarketDataInstrumentKind {
    match kind {
        "perpetual" => MarketDataInstrumentKind::Perpetual,
        _ => MarketDataInstrumentKind::Spot,
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyOrderBook>()?;
    parent.add_class::<PyOrderBookManager>()?;
    parent.add_function(wrap_pyfunction!(build_order_book_manager, parent)?)?;
    Ok(())
}

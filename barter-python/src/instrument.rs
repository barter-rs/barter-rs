use barter_instrument::{
    Side, Underlying,
    asset::Asset,
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{
        Instrument,
        kind::{InstrumentKind, perpetual::PerpetualContract},
        quote::InstrumentQuoteAsset,
    },
};
use rust_decimal_macros::dec;
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Side
// ---------------------------------------------------------------------------

#[pyclass(name = "Side", module = "barter")]
#[derive(Debug, Clone)]
pub struct PySide(pub Side);

#[pymethods]
impl PySide {
    #[classattr]
    const BUY: &str = "buy";
    #[classattr]
    const SELL: &str = "sell";

    #[new]
    fn new(value: &str) -> PyResult<Self> {
        match value.to_lowercase().as_str() {
            "buy" | "b" => Ok(Self(Side::Buy)),
            "sell" | "s" => Ok(Self(Side::Sell)),
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "invalid side: {other}"
            ))),
        }
    }

    fn __repr__(&self) -> String {
        match self.0 {
            Side::Buy => "Side.BUY".into(),
            Side::Sell => "Side.SELL".into(),
        }
    }

    fn __str__(&self) -> &'static str {
        match self.0 {
            Side::Buy => "buy",
            Side::Sell => "sell",
        }
    }

    fn __eq__(&self, other: &PySide) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        match self.0 {
            Side::Buy => 0,
            Side::Sell => 1,
        }
    }
}

// ---------------------------------------------------------------------------
// ExchangeId
// ---------------------------------------------------------------------------

#[pyclass(name = "ExchangeId", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyExchangeId(pub ExchangeId);

#[pymethods]
impl PyExchangeId {
    #[new]
    fn new(name: &str) -> PyResult<Self> {
        let id: ExchangeId = serde_json::from_value(serde_json::Value::String(name.to_string()))
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("unknown exchange: {e}"))
            })?;
        Ok(Self(id))
    }

    fn __repr__(&self) -> String {
        format!("ExchangeId('{}')", self.__str__())
    }

    fn __str__(&self) -> String {
        serde_json::to_value(&self.0)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self.0))
    }

    fn __eq__(&self, other: &PyExchangeId) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.0.hash(&mut h);
        h.finish()
    }

    // Common exchange constants
    #[classattr]
    fn BINANCE_SPOT() -> Self {
        Self(ExchangeId::BinanceSpot)
    }
    #[classattr]
    fn BINANCE_FUTURES_USD() -> Self {
        Self(ExchangeId::BinanceFuturesUsd)
    }
    #[classattr]
    fn OKX() -> Self {
        Self(ExchangeId::Okx)
    }
    #[classattr]
    fn COINBASE() -> Self {
        Self(ExchangeId::Coinbase)
    }
    #[classattr]
    fn KRAKEN() -> Self {
        Self(ExchangeId::Kraken)
    }
    #[classattr]
    fn BYBIT_PERPETUALS_USD() -> Self {
        Self(ExchangeId::BybitPerpetualsUsd)
    }
    #[classattr]
    fn BITFINEX() -> Self {
        Self(ExchangeId::Bitfinex)
    }
    #[classattr]
    fn GATEIO_SPOT() -> Self {
        Self(ExchangeId::GateioSpot)
    }
    #[classattr]
    fn GATEIO_PERPETUALS_USD() -> Self {
        Self(ExchangeId::GateioPerpetualsUsd)
    }
    #[classattr]
    fn BITMEX() -> Self {
        Self(ExchangeId::Bitmex)
    }
}

// ---------------------------------------------------------------------------
// Instrument
// ---------------------------------------------------------------------------

#[pyclass(name = "Instrument", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyInstrument {
    pub inner: Instrument<ExchangeId, Asset>,
}

#[pymethods]
impl PyInstrument {
    /// Create a new Spot instrument.
    #[staticmethod]
    fn spot(
        exchange: &PyExchangeId,
        name_internal: &str,
        name_exchange: &str,
        base: &str,
        quote: &str,
    ) -> Self {
        Self {
            inner: Instrument::spot(
                exchange.0,
                name_internal,
                name_exchange,
                Underlying::new(base, quote),
                None,
            ),
        }
    }

    /// Create a new instrument with a specific kind.
    ///
    /// Args:
    ///     exchange: ExchangeId
    ///     name_internal: str
    ///     name_exchange: str
    ///     base: str
    ///     quote: str
    ///     kind: str — "spot", "perpetual"
    #[staticmethod]
    #[pyo3(signature = (exchange, name_internal, name_exchange, base, quote, kind="spot"))]
    fn new_instrument(
        exchange: &PyExchangeId,
        name_internal: &str,
        name_exchange: &str,
        base: &str,
        quote: &str,
        kind: &str,
    ) -> Self {
        let instrument_kind = match kind {
            "perpetual" => InstrumentKind::Perpetual(PerpetualContract {
                contract_size: dec!(1),
                settlement_asset: Asset::from("usdt"),
            }),
            _ => InstrumentKind::Spot,
        };
        Self {
            inner: Instrument::new(
                exchange.0,
                name_internal,
                name_exchange,
                Underlying::new(base, quote),
                InstrumentQuoteAsset::UnderlyingQuote,
                instrument_kind,
                None,
            ),
        }
    }

    #[getter]
    fn exchange(&self) -> PyExchangeId {
        PyExchangeId(self.inner.exchange)
    }

    #[getter]
    fn name_internal(&self) -> String {
        self.inner.name_internal.to_string()
    }

    #[getter]
    fn name_exchange(&self) -> String {
        self.inner.name_exchange.to_string()
    }

    #[getter]
    fn base(&self) -> String {
        self.inner.underlying.base.name_internal.to_string()
    }

    #[getter]
    fn quote_asset(&self) -> String {
        self.inner.underlying.quote.name_internal.to_string()
    }

    #[getter]
    fn kind(&self) -> String {
        match &self.inner.kind {
            InstrumentKind::Spot => "spot".into(),
            InstrumentKind::Perpetual(_) => "perpetual".into(),
            InstrumentKind::Future(_) => "future".into(),
            InstrumentKind::Option(_) => "option".into(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Instrument(exchange='{}', name='{}', kind='{}', base='{}', quote='{}')",
            PyExchangeId(self.inner.exchange).__str__(),
            self.inner.name_exchange,
            self.kind(),
            self.inner.underlying.base.name_internal,
            self.inner.underlying.quote.name_internal,
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{}:{} ({})",
            PyExchangeId(self.inner.exchange).__str__(),
            self.inner.name_exchange,
            self.kind()
        )
    }

    /// Serialise to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Deserialise from JSON string.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: Instrument<ExchangeId, Asset> = serde_json::from_str(json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }
}

// ---------------------------------------------------------------------------
// IndexedInstruments
// ---------------------------------------------------------------------------

#[pyclass(name = "IndexedInstruments", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyIndexedInstruments {
    pub inner: IndexedInstruments,
}

#[pymethods]
impl PyIndexedInstruments {
    /// Build IndexedInstruments from a list of Instrument objects.
    #[new]
    fn new(instruments: Vec<PyInstrument>) -> Self {
        let inner = IndexedInstruments::new(
            instruments.into_iter().map(|i| i.inner),
        );
        Self { inner }
    }

    /// Build from a JSON config string (list of instrument objects).
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let instruments: Vec<Instrument<ExchangeId, Asset>> = serde_json::from_str(json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: IndexedInstruments::new(instruments),
        })
    }

    /// Number of instruments.
    fn __len__(&self) -> usize {
        self.inner.instruments().len()
    }

    /// Get the number of exchanges.
    #[getter]
    fn num_exchanges(&self) -> usize {
        self.inner.exchanges().len()
    }

    /// Get the number of assets.
    #[getter]
    fn num_assets(&self) -> usize {
        self.inner.assets().len()
    }

    /// List all exchanges as (index, name) tuples.
    fn exchanges(&self) -> Vec<(usize, String)> {
        self.inner
            .exchanges()
            .iter()
            .map(|keyed| {
                (
                    keyed.key.0,
                    serde_json::to_value(&keyed.value)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| format!("{:?}", keyed.value)),
                )
            })
            .collect()
    }

    /// List all instruments as (index, name_internal, name_exchange, exchange, kind) tuples.
    fn instruments(&self) -> Vec<(usize, String, String, String, String)> {
        self.inner
            .instruments()
            .iter()
            .map(|keyed| {
                let inst = &keyed.value;
                let exchange_str = serde_json::to_value(&inst.exchange.value)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| format!("{:?}", inst.exchange.value));
                let kind = match &inst.kind {
                    InstrumentKind::Spot => "spot",
                    InstrumentKind::Perpetual(_) => "perpetual",
                    InstrumentKind::Future(_) => "future",
                    InstrumentKind::Option(_) => "option",
                };
                (
                    keyed.key.0,
                    inst.name_internal.to_string(),
                    inst.name_exchange.to_string(),
                    exchange_str,
                    kind.to_string(),
                )
            })
            .collect()
    }

    /// Find the exchange index for a given ExchangeId.
    fn find_exchange_index(&self, exchange: &PyExchangeId) -> PyResult<usize> {
        self.inner
            .find_exchange_index(exchange.0)
            .map(|idx| idx.0)
            .map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "IndexedInstruments(exchanges={}, assets={}, instruments={})",
            self.inner.exchanges().len(),
            self.inner.assets().len(),
            self.inner.instruments().len(),
        )
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PySide>()?;
    parent.add_class::<PyExchangeId>()?;
    parent.add_class::<PyInstrument>()?;
    parent.add_class::<PyIndexedInstruments>()?;
    Ok(())
}

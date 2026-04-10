use crate::decimal::{decimal_to_f64, decimal_to_py, py_to_decimal};
use crate::instrument::PySide;
use barter_execution::balance::Balance;
use barter_instrument::Side;
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Balance
// ---------------------------------------------------------------------------

#[pyclass(name = "Balance", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyBalance {
    pub inner: Balance,
}

#[pymethods]
impl PyBalance {
    #[new]
    fn new(total: &Bound<'_, PyAny>, free: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: Balance {
                total: py_to_decimal(total)?,
                free: py_to_decimal(free)?,
            },
        })
    }

    #[getter]
    fn total(&self, py: Python<'_>) -> PyResult<PyObject> {
        decimal_to_py(py, &self.inner.total)
    }

    #[getter]
    fn free(&self, py: Python<'_>) -> PyResult<PyObject> {
        decimal_to_py(py, &self.inner.free)
    }

    /// Returns total - free (amount currently locked in orders).
    #[getter]
    fn locked(&self, py: Python<'_>) -> PyResult<PyObject> {
        let locked = self.inner.total - self.inner.free;
        decimal_to_py(py, &locked)
    }

    /// Convenience: total as float.
    #[getter]
    fn total_f64(&self) -> f64 {
        decimal_to_f64(&self.inner.total)
    }

    /// Convenience: free as float.
    #[getter]
    fn free_f64(&self) -> f64 {
        decimal_to_f64(&self.inner.free)
    }

    fn __repr__(&self) -> String {
        format!(
            "Balance(total={}, free={})",
            self.inner.total, self.inner.free
        )
    }

    fn __eq__(&self, other: &PyBalance) -> bool {
        self.inner == other.inner
    }
}

// ---------------------------------------------------------------------------
// PublicTrade (from barter-data, but logically belongs with execution types)
// ---------------------------------------------------------------------------

#[pyclass(name = "PublicTrade", module = "barter")]
#[derive(Debug, Clone)]
pub struct PyPublicTrade {
    pub id: String,
    pub price: f64,
    pub amount: f64,
    pub side: Side,
}

#[pymethods]
impl PyPublicTrade {
    #[getter]
    fn id(&self) -> &str {
        &self.id
    }

    #[getter]
    fn price(&self) -> f64 {
        self.price
    }

    #[getter]
    fn amount(&self) -> f64 {
        self.amount
    }

    #[getter]
    fn side(&self) -> PySide {
        PySide(self.side)
    }

    fn __repr__(&self) -> String {
        format!(
            "PublicTrade(id='{}', price={}, amount={}, side='{}')",
            self.id,
            self.price,
            self.amount,
            match self.side {
                Side::Buy => "buy",
                Side::Sell => "sell",
            }
        )
    }
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyBalance>()?;
    parent.add_class::<PyPublicTrade>()?;
    Ok(())
}

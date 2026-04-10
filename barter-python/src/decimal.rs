use pyo3::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Convert a `rust_decimal::Decimal` to a Python `decimal.Decimal`.
pub fn decimal_to_py(py: Python<'_>, d: &Decimal) -> PyResult<PyObject> {
    let decimal_mod = py.import("decimal")?;
    let decimal_cls = decimal_mod.getattr("Decimal")?;
    let s = d.to_string();
    let py_dec = decimal_cls.call1((s,))?;
    Ok(py_dec.into())
}

/// Convert a Python object to a `rust_decimal::Decimal`.
///
/// Accepts Python `decimal.Decimal`, `float`, `int`, or `str`.
pub fn py_to_decimal(obj: &Bound<'_, PyAny>) -> PyResult<Decimal> {
    let s: String = obj.str()?.to_string();
    Decimal::from_str(&s).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("cannot convert to Decimal: {e}"))
    })
}

/// Convert a `Decimal` to a Python `float` (lossy but convenient).
pub fn decimal_to_f64(d: &Decimal) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f64().unwrap_or(f64::NAN)
}

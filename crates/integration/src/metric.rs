use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize)]
pub struct Metric {
    /// Metric name.
    pub name: &'static str,

    /// Milliseconds since the Unix epoch.
    pub time: u64,

    /// Key-Value pairs to categorise the Metric.
    pub tags: Vec<Tag>,

    /// Observed measurements.
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct Tag {
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Serialize)]
pub struct Field {
    pub key: &'static str,
    pub value: Value,
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Deserialize, Serialize)]
pub enum Value {
    Float(f64),
    Int(i64),
    UInt(u64),
    Bool(bool),
    String(String),
}

impl<S> From<(&'static str, S)> for Tag
where
    S: Into<String>,
{
    fn from((key, value): (&'static str, S)) -> Self {
        Self::new(key, value)
    }
}

impl Tag {
    pub fn new<S>(key: &'static str, value: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            key,
            value: value.into(),
        }
    }
}

impl<S> From<(&'static str, S)> for Field
where
    S: Into<Value>,
{
    fn from((key, value): (&'static str, S)) -> Self {
        Self::new(key, value)
    }
}

impl Field {
    pub fn new<S>(key: &'static str, value: S) -> Self
    where
        S: Into<Value>,
    {
        Self {
            key,
            value: value.into(),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::UInt(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

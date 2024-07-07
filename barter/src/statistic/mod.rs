use chrono::Duration;
use serde::{Deserialize, Deserializer, Serializer};

pub mod algorithm;
pub mod dispersion;
pub mod error;
pub mod metric;
pub mod summary;

/// Serialize a [`Duration`] into a `u64` representing the associated seconds.
pub fn se_duration_as_secs<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(duration.num_seconds())
}

/// Deserialize a number representing seconds into a [`Duration`]
pub fn de_duration_from_secs<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds: i64 = Deserialize::deserialize(deserializer)?;
    Ok(Duration::seconds(seconds))
}

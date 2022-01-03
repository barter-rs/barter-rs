use chrono::Duration;
use serde::Serializer;

pub mod algorithm;
pub mod dispersion;
pub mod error;
pub mod metric;
pub mod summary;

/// Serialize a [`Duration`] into a `u64` representing the associated seconds.
pub fn se_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(duration.num_seconds())
}

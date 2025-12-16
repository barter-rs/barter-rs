use crate::serde::se::error::SeError;
use tracing::debug;

pub mod error;

#[derive(Debug, Default)]
pub struct SeString;

impl SeString {
    pub fn se_string<T>(input: &T) -> Result<String, SeError>
    where
        T: std::fmt::Debug + serde::Serialize,
    {
        serde_json::to_string(input)
            .map(|string| {
                debug!(
                    payload = %string,
                    input_type = %std::any::type_name::<T>(),
                    target_type = "String",
                    "successfully serialised via Serde"
                );
                string
            })
            .map_err(|error| {
                debug!(
                    %error,
                    payload = ?input,
                    target_type = "String",
                    "failed to serialise via SerDe"
                );
                SeError::Serde(error)
            })
    }
}

#[derive(Debug, Default)]
pub struct SeBytes;

impl SeBytes {
    pub fn se_bytes<T>(input: &T) -> Result<bytes::Bytes, SeError>
    where
        T: std::fmt::Debug + serde::Serialize,
    {
        serde_json::to_vec(input)
            .map(|bytes| {
                debug!(
                    payload = ?bytes,
                    input_type = %std::any::type_name::<T>(),
                    target_type = "bytes::Bytes",
                    "successfully serialised to via Serde"
                );
                bytes::Bytes::from(bytes)
            })
            .map_err(|error| {
                debug!(
                    %error,
                    payload = ?input,
                    target_type = "bytes::Bytes",
                    "failed to serialise via SerDe"
                );
                SeError::Serde(error)
            })
    }
}

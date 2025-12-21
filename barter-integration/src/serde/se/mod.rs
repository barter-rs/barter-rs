use crate::serde::se::error::SeError;
use tracing::debug;

pub mod error;

/// JSON String serialiser.
#[derive(Debug, Default)]
pub struct SeJsonString;

impl SeJsonString {
    /// Serialises the input type into a valid JSON string.
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

/// JSON bytes serialiser.
#[derive(Debug, Default)]
pub struct SeJsonBytes;

impl SeJsonBytes {
    /// Serialises the input type into valid JSON bytes.
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
                    "successfully serialised via Serde"
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

    /// Serialises the input type into the valid JSON byte writer.
    pub fn se_bytes_writer<T>(writer: impl std::io::Write, input: &T) -> Result<(), SeError>
    where
        T: std::fmt::Debug + serde::Serialize,
    {
        serde_json::to_writer(writer, input)
            .map(|_| {
                debug!(
                    payload = ?input,
                    target_type = "std::io::Write",
                    input_type = %std::any::type_name::<T>(),
                    "successfully serialised to writer via Serde"
                );
            })
            .map_err(|error| {
                debug!(
                    %error,
                    payload = ?input,
                    target_type = "std::io::Writer",
                    "failed to serialise to writer via SerDe"
                );
                SeError::Serde(error)
            })
    }
}

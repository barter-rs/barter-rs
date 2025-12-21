use crate::serde::se::error::SeError;
use serde::ser::SerializeSeq;
use tracing::debug;

pub mod error;

/// Serialise a generic element T as a `Vec<T>`.
pub fn se_element_to_vector<T, S>(element: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: serde::Serialize,
{
    let mut sequence = serializer.serialize_seq(Some(1))?;
    sequence.serialize_element(&element)?;
    sequence.end()
}

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
                    payload = %String::from_utf8_lossy(&bytes),
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
    pub fn se_bytes_writer<T>(writer: impl std::fmt::Write, input: &T) -> Result<(), SeError>
    where
        T: std::fmt::Debug + serde::Serialize,
    {
        serde_json::to_writer(writer, input)
            .map(|_| {
                debug!(
                    payload = ?input,
                    target_type = "std::fmt::Write",
                    input_type = %std::any::type_name::<T>(),
                    "successfully serialised to writer via Serde"
                );
            })
            .map_err(|error| {
                debug!(
                    %error,
                    payload = ?input,
                    target_type = "std::io::Write",
                    "failed to serialise to writer via SerDe"
                );
                SeError::Serde(error)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_se_json_string() {
        let input = serde_json::json!({"key": "value"});
        let result = SeJsonString::se_string(&input).unwrap();
        assert_eq!(result, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_se_json_bytes() {
        let input = serde_json::json!({"key": "value"});
        let result = SeJsonBytes::se_bytes(&input).unwrap();
        assert_eq!(result.as_ref(), br#"{"key":"value"}"#);
    }

    #[test]
    fn test_se_json_bytes_writer() {
        let input = serde_json::json!({"key": "value"});
        let mut buf = Vec::new();
        SeJsonBytes::se_bytes_writer(&mut buf, &input).unwrap();
        assert_eq!(buf, br#"{"key":"value"}"#);
    }

    #[test]
    fn test_se_element_to_vector() {
        #[derive(serde::Serialize)]
        struct Wrapper(#[serde(serialize_with = "se_element_to_vector")] u32);

        let json = serde_json::to_string(&Wrapper(42)).unwrap();
        assert_eq!(json, "[42]");
    }
}

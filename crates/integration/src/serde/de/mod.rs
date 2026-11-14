use crate::serde::de::error::{DeBinaryError, DeBinaryErrorKind};
use tracing::debug;

/// Deserialisation error variants.
pub mod error;

/// Deserialisation utilities.
mod util;

// Re-export deserialisation utilities.
pub use util::*;

/// Trait for types that can deserialise input payload data (eg/ strings, bytes, etc.) into
/// structured Rust types.
pub trait Deserialiser<Input, Output> {
    type Error;

    /// Deserialises the `Input` into the `Output`.
    fn deserialise(input: Input) -> Result<Output, Self::Error>;
}

/// JSON deserialiser.
#[derive(Debug, Default)]
pub struct DeJson;

impl<'a, Output> Deserialiser<&'a [u8], Output> for DeJson
where
    Output: serde::Deserialize<'a> + 'a,
{
    type Error = DeBinaryError;

    fn deserialise(input: &'a [u8]) -> Result<Output, Self::Error> {
        Self::de_bytes(input)
    }
}

impl<Output> Deserialiser<bytes::Bytes, Output> for DeJson
where
    Output: for<'a> serde::Deserialize<'a>,
{
    type Error = DeBinaryError;

    fn deserialise(input: bytes::Bytes) -> Result<Output, Self::Error> {
        Self::de_bytes(input.as_ref())
    }
}

impl DeJson {
    /// Deserialises a byte slice into the target `Output` type using [`serde_json`].
    pub fn de_bytes<'a, Output>(input: &'a [u8]) -> Result<Output, DeBinaryError>
    where
        Output: serde::Deserialize<'a> + 'a,
    {
        serde_json::from_slice::<Output>(input).map_err(|error| {
            let input_str = std::str::from_utf8(input).unwrap_or("<invalid UTF-8>");

            debug!(
                %error,
                ?input,
                %input_str,
                input_type = "&[u8]",
                target_type = %std::any::type_name::<Output>(),
                "failed to deserialise via SerDe"
            );

            DeBinaryError {
                payload: input.to_vec(),
                kind: DeBinaryErrorKind::Serde(error),
            }
        })
    }
}

/// Protobuf deserialiser.
#[derive(Debug, Default)]
pub struct DeProtobuf;

impl<'a, Output> Deserialiser<&'a [u8], Output> for DeProtobuf
where
    Output: prost::Message + Default,
{
    type Error = DeBinaryError;

    fn deserialise(input: &'a [u8]) -> Result<Output, Self::Error> {
        Self::decode_bytes(input)
    }
}

impl<Output> Deserialiser<bytes::Bytes, Output> for DeProtobuf
where
    Output: prost::Message + Default,
{
    type Error = DeBinaryError;

    fn deserialise(input: bytes::Bytes) -> Result<Output, Self::Error> {
        Self::decode_bytes(input.as_ref())
    }
}

impl DeProtobuf {
    /// Decodes a byte slice into the target `Output` type using [`prost`].
    pub fn decode_bytes<Output>(input: &[u8]) -> Result<Output, DeBinaryError>
    where
        Output: prost::Message + Default,
    {
        Output::decode(input).map_err(|error| {
            debug!(
                %error,
                ?input,
                target_type = %std::any::type_name::<Output>(),
                input_type = "&[u8]",
                "failed to deserialise via prost::Message"
            );

            DeBinaryError {
                payload: input.to_vec(),
                kind: DeBinaryErrorKind::Proto(error),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_de_json_de_bytes_valid() {
        let input = br#"{"a":1,"b":"hello"}"#;
        let result: std::collections::HashMap<String, serde_json::Value> =
            DeJson::de_bytes(input).unwrap();
        assert_eq!(result["a"], serde_json::json!(1));
        assert_eq!(result["b"], serde_json::json!("hello"));
    }

    #[test]
    fn test_de_json_de_bytes_invalid_json() {
        let input = b"not valid json";
        let result = DeJson::de_bytes::<serde_json::Value>(input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.payload, input.to_vec());
        assert!(matches!(err.kind, DeBinaryErrorKind::Serde(_)));
    }

    #[test]
    fn test_de_json_deserialiser_trait_bytes() {
        let input = bytes::Bytes::from(r#"42"#);
        let result: u64 = <DeJson as Deserialiser<bytes::Bytes, _>>::deserialise(input).unwrap();
        assert_eq!(result, 42);
    }
}

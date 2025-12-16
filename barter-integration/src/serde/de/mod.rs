use crate::serde::de::error::{DeBinaryError, DeBinaryErrorKind};
use tracing::debug;

pub mod error;
pub mod util;

pub trait Deserialiser<Input, Output> {
    type Error;

    fn deserialise(input: Input) -> Result<Output, Self::Error>;
}

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
    pub fn de_bytes<'a, Output>(input: &'a [u8]) -> Result<Output, DeBinaryError>
    where
        Output: serde::Deserialize<'a> + 'a,
    {
        serde_json::from_slice::<Output>(input).map_err(|error| {
            let input_str =
                String::from_utf8(input.to_vec()).unwrap_or_else(|error| error.to_string());

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

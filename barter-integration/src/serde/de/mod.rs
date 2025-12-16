use crate::serde::de::error::{DeBinaryError, DeBinaryErrorKind};
use tracing::debug;

pub mod error;
pub mod util;

pub trait Deserialiser<Input> {
    type Output;
    type Error;

    fn deserialise(input: Input) -> Result<Self::Output, Self::Error>;
}

#[derive(Debug)]
pub struct DeJson<Output>(std::marker::PhantomData<Output>);

impl<T> Default for DeJson<T> {
    fn default() -> Self {
        Self(<_>::default())
    }
}

impl<'a, Output> Deserialiser<&'a [u8]> for DeJson<Output>
where
    Output: serde::Deserialize<'a> + 'a,
{
    type Output = Output;
    type Error = DeBinaryError;

    fn deserialise(input: &'a [u8]) -> Result<Self::Output, Self::Error> {
        Self::de_bytes(input)
    }
}

impl<Output> Deserialiser<bytes::Bytes> for DeJson<Output>
where
    Output: for<'a> serde::Deserialize<'a>,
{
    type Output = Output;
    type Error = DeBinaryError;

    fn deserialise(input: bytes::Bytes) -> Result<Self::Output, Self::Error> {
        Self::de_bytes(input.as_ref())
    }
}

impl<Output> DeJson<Output> {
    pub fn de_bytes<'a>(input: &'a [u8]) -> Result<Output, DeBinaryError>
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

#[derive(Debug)]
pub struct DeProtobuf<T>(std::marker::PhantomData<T>);

impl<T> Default for DeProtobuf<T> {
    fn default() -> Self {
        Self(<_>::default())
    }
}

impl<'a, Output> Deserialiser<&'a [u8]> for DeProtobuf<Output>
where
    Output: prost::Message + Default,
{
    type Output = Output;
    type Error = DeBinaryError;

    fn deserialise(input: &'a [u8]) -> Result<Self::Output, Self::Error> {
        Self::decode_bytes(input)
    }
}

impl<Output> Deserialiser<bytes::Bytes> for DeProtobuf<Output>
where
    Output: prost::Message + Default,
{
    type Output = Output;
    type Error = DeBinaryError;

    fn deserialise(input: bytes::Bytes) -> Result<Self::Output, Self::Error> {
        Self::decode_bytes(input.as_ref())
    }
}

impl<Output> DeProtobuf<Output> {
    pub fn decode_bytes(input: &[u8]) -> Result<Output, DeBinaryError>
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

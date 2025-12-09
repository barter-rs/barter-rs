use crate::Transformer;
use serde::Serialize;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use tracing::debug;

/// Utilities to assist with deserialisation.
pub mod de;

#[derive(Debug)]
pub struct SeBinaryError {
    error: serde_json::Error,
}

#[derive(Debug)]
pub struct SeTransformer<T>(std::marker::PhantomData<T>);

impl<T> Default for SeTransformer<T> {
    fn default() -> Self {
        Self(<_>::default())
    }
}

impl<'t, T> Transformer<&'t T> for SeTransformer<T>
where
    T: std::fmt::Debug + serde::Serialize + 't,
{
    type Output<'a>
        = Result<bytes::Bytes, SeBinaryError>
    where
        &'t T: 'a;

    fn transform<'a>(input: &T) -> impl IntoIterator<Item = Self::Output<'a>> + 'a
    where
        &'t T: 'a,
    {
        let output = serde_json::to_vec(input)
            .map(bytes::Bytes::from)
            .map_err(|error| {
                debug!(
                    %error,
                    payload = ?input,
                    target_type = "bytes::Bytes",
                    "failed to serialise to bytes::Bytes via SerDe"
                );

                SeBinaryError { error }
            });

        std::iter::once(output)
    }
}

#[derive(Debug)]
pub struct DeBinaryError {
    payload: Vec<u8>,
    kind: DeBinaryErrorKind,
}

#[derive(Debug)]
pub enum DeBinaryErrorKind {
    Serde(serde_json::Error),
    Proto(prost::DecodeError),
}

#[derive(Debug)]
pub struct DeTransformer<T>(std::marker::PhantomData<T>);

impl<T> Default for DeTransformer<T> {
    fn default() -> Self {
        Self(<_>::default())
    }
}

impl<T> Transformer<bytes::Bytes> for DeTransformer<T>
where
    for<'de> T: serde::Deserialize<'de> + 'de,
{
    type Output<'a> = Result<T, DeBinaryError>;

    fn transform<'a>(payload: bytes::Bytes) -> impl IntoIterator<Item = Self::Output<'a>> + 'a {
        let result = Self::de_bytes(payload.as_ref());
        std::iter::once(result)
    }
}

impl<T> DeTransformer<T> {
    pub fn de_bytes<'a>(payload: &'a [u8]) -> Result<T, DeBinaryError>
    where
        T: serde::Deserialize<'a> + 'a,
    {
        serde_json::from_slice::<T>(payload).map_err(|error| {
            let payload_str =
                String::from_utf8(payload.to_vec()).unwrap_or_else(|error| error.to_string());

            debug!(
                %error,
                ?payload,
                %payload_str,
                target_type = %std::any::type_name::<T>(),
                "failed to deserialise bytes::Bytes via SerDe"
            );

            DeBinaryError {
                payload: payload.to_vec(),
                kind: DeBinaryErrorKind::Serde(error),
            }
        })
    }
}

#[derive(Debug)]
pub struct DeProtobufTransformer<T>(std::marker::PhantomData<T>);

impl<T> Default for DeProtobufTransformer<T> {
    fn default() -> Self {
        Self(<_>::default())
    }
}

impl<T> Transformer<bytes::Bytes> for DeProtobufTransformer<T>
where
    for<'a> T: prost::Message + Default + 'a,
{
    type Output<'a> = Result<T, DeBinaryError>;

    fn transform<'a>(payload: bytes::Bytes) -> impl IntoIterator<Item = Self::Output<'a>> + 'a {
        let result = Self::decode_bytes(payload.as_ref());
        std::iter::once(result)
    }
}

impl<T> DeProtobufTransformer<T> {
    pub fn decode_bytes(payload: &[u8]) -> Result<T, DeBinaryError>
    where
        T: prost::Message + Default,
    {
        T::decode(payload).map_err(|error| {
            debug!(
                %error,
                ?payload,
                target_type = %std::any::type_name::<T>(),
                "failed to deserialise bytes::Bytes via prost::Message"
            );

            DeBinaryError {
                payload: payload.to_vec(),
                kind: DeBinaryErrorKind::Proto(error),
            }
        })
    }
}

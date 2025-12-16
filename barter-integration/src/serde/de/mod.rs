use crate::Transformer;
use tracing::debug;

/// Utilities to assist with deserialisation.
pub mod util;

#[derive(Debug)]
pub struct DeBinaryError {
    pub payload: Vec<u8>,
    pub kind: DeBinaryErrorKind,
}

#[derive(Debug)]
pub enum DeBinaryErrorKind {
    Serde(serde_json::Error),
    Proto(prost::DecodeError),
}

#[derive(Debug)]
pub struct DeJson<T>(std::marker::PhantomData<T>);

impl<T> Default for DeJson<T> {
    fn default() -> Self {
        Self(<_>::default())
    }
}

impl<T> Transformer<bytes::Bytes> for DeJson<T>
where
    for<'de> T: serde::Deserialize<'de> + Send + 'de,
{
    type Output<'a> = Result<T, DeBinaryError>;

    fn transform<'a>(
        payload: bytes::Bytes,
    ) -> impl IntoIterator<Item = Self::Output<'a>, IntoIter: Send> + 'a {
        let result = Self::de_bytes(payload.as_ref());
        std::iter::once(result)
    }
}

impl<'de, T> Transformer<&'de [u8]> for DeJson<T>
where
    T: serde::Deserialize<'de> + Send + 'de,
{
    type Output<'a>
        = Result<T, DeBinaryError>
    where
        'de: 'a;

    fn transform<'a>(
        payload: &'de [u8],
    ) -> impl IntoIterator<Item = Self::Output<'a>, IntoIter: Send> + 'a
    where
        'de: 'a,
        T: 'a,
    {
        let result = Self::de_bytes(payload);
        std::iter::once(result)
    }
}

impl<T> DeJson<T> {
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
                input_type = "bytes::Bytes",
                target_type = %std::any::type_name::<T>(),
                "failed to deserialise via SerDe"
            );

            DeBinaryError {
                payload: payload.to_vec(),
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

impl<T> Transformer<bytes::Bytes> for DeProtobuf<T>
where
    for<'a> T: prost::Message + Default + 'a,
{
    type Output<'a> = Result<T, DeBinaryError>;

    fn transform<'a>(
        payload: bytes::Bytes,
    ) -> impl IntoIterator<Item = Self::Output<'a>, IntoIter: Send> + 'a {
        let result = Self::decode_bytes(payload.as_ref());
        std::iter::once(result)
    }
}

impl<T> DeProtobuf<T> {
    pub fn decode_bytes(payload: &[u8]) -> Result<T, DeBinaryError>
    where
        T: prost::Message + Default,
    {
        T::decode(payload).map_err(|error| {
            debug!(
                %error,
                ?payload,
                target_type = %std::any::type_name::<T>(),
                input_type = "bytes::Bytes",
                "failed to deserialise via prost::Message"
            );

            DeBinaryError {
                payload: payload.to_vec(),
                kind: DeBinaryErrorKind::Proto(error),
            }
        })
    }
}

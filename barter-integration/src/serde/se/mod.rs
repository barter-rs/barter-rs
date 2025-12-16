use crate::Transformer;
use tracing::debug;

#[derive(Debug, thiserror::Error)]
pub enum SeError {
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Default)]
pub struct SeString;

impl<T> Transformer<T> for SeString
where
    T: std::fmt::Debug + serde::Serialize,
{
    type Output<'a>
        = Result<String, SeError>
    where
        T: 'a;

    fn transform<'a>(input: T) -> impl IntoIterator<Item = Self::Output<'a>, IntoIter: Send> + 'a
    where
        T: 'a,
    {
        let output = Self::se_string(&input);
        std::iter::once(output)
    }
}

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

impl<'t, T> Transformer<&'t T> for SeBytes
where
    T: std::fmt::Debug + serde::Serialize + 't,
{
    type Output<'a>
        = Result<bytes::Bytes, SeError>
    where
        &'t T: 'a;

    fn transform<'a>(
        input: &'t T,
    ) -> impl IntoIterator<Item = Self::Output<'a>, IntoIter: Send> + 'a
    where
        &'t T: 'a,
    {
        let output = Self::se_bytes(input);
        std::iter::once(output)
    }
}

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

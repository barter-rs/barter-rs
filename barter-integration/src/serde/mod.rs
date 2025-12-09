use tracing::debug;

/// Utilities to assist with deserialisation.
pub mod de;

#[derive(Debug, Default)]
pub struct SerdeTransformer;

impl SerdeTransformer {
    pub fn transform<'a, T>(
        payload: &'a [u8],
    ) -> impl IntoIterator<Item = Result<T, DeBinaryError>> + 'a
    where
        T: serde::Deserialize<'a> + 'a,
    {
        let output = serde_json::from_slice::<T>(payload).map_err(|error| {
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
                error,
                payload: payload.to_vec(),
            }
        });

        std::iter::once(output)
    }
}

#[derive(Debug)]
pub struct DeBinaryError {
    pub error: serde_json::error::Error,
    pub payload: Vec<u8>,
}

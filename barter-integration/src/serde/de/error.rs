/// Deserialisation error including the input payload.
#[derive(Debug, thiserror::Error)]
#[error("failed to deserialise binary payload ({} bytes): {kind}", payload.len())]
pub struct DeBinaryError {
    pub payload: Vec<u8>,
    pub kind: DeBinaryErrorKind,
}

/// Deserialisation error kinds.
#[derive(Debug, thiserror::Error)]
pub enum DeBinaryErrorKind {
    /// `serde_json` error.
    #[error("serde deserialisation: {0}")]
    Serde(#[from] serde_json::Error),

    /// `prost` error.
    #[error("protobuf deserialisation: {0}")]
    Proto(#[from] prost::DecodeError),
}

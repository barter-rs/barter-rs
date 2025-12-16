#[derive(Debug, thiserror::Error)]
#[error("failed to deserialise binary payload ({} bytes): {kind}", payload.len())]
pub struct DeBinaryError {
    pub payload: Vec<u8>,
    pub kind: DeBinaryErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum DeBinaryErrorKind {
    #[error("Serde deserialisation: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Protobuf deserialisation: {0}")]
    Proto(#[from] prost::DecodeError),
}

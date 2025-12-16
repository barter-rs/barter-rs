#[derive(Debug, thiserror::Error)]
pub enum SeError {
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

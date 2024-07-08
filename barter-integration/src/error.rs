use crate::model::SubscriptionId;
use reqwest::Error;
use thiserror::Error;

/// All socket IO related errors generated in `barter-integration`.
#[derive(Debug, Error)]
pub enum SocketError {
    #[error("Sink error")]
    Sink,

    #[error("Deserialising JSON error: {error} for payload: {payload}")]
    Deserialise {
        error: serde_json::Error,
        payload: String,
    },

    #[error("Deserialising JSON error: {error} for binary payload: {payload:?}")]
    DeserialiseBinary {
        error: serde_json::Error,
        payload: Vec<u8>,
    },

    #[error("Serialising JSON error: {0}")]
    Serialise(serde_json::Error),

    #[error("SerDe Query String serialisation error: {0}")]
    QueryParams(#[from] serde_qs::Error),

    #[error("SerDe url encoding serialisation error: {0}")]
    UrlEncoded(#[from] serde_urlencoded::ser::Error),

    #[error("error parsing Url: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("error subscribing to resources over the socket: {0}")]
    Subscribe(String),

    #[error("ExchangeStream terminated with closing frame: {0}")]
    Terminated(String),

    #[error("{entity} does not support: {item}")]
    Unsupported { entity: &'static str, item: String },

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("HTTP error: {0}")]
    Http(reqwest::Error),

    #[error("HTTP request timed out")]
    HttpTimeout(reqwest::Error),

    /// REST http response error
    #[error("HTTP response (status={0}) error: {1}")]
    HttpResponse(reqwest::StatusCode, String),

    #[error("consumed unidentifiable message: {0}")]
    Unidentifiable(SubscriptionId),

    #[error("consumed error message from exchange: {0}")]
    Exchange(String),
}

impl From<reqwest::Error> for SocketError {
    fn from(error: Error) -> Self {
        match error {
            error if error.is_timeout() => SocketError::HttpTimeout(error),
            error => SocketError::Http(error),
        }
    }
}

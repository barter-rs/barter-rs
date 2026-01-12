use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KrakenFuturesMessage<T> {
    pub feed: String,
    pub product_id: String,
    #[serde(flatten)]
    pub payload: T,
}

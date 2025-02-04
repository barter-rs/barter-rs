use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_with::{serde_as, TimestampMilliSeconds};

pub mod parser;
pub mod requests;
pub mod signer;

/// Generic response from Bybit used for all REST responses.
#[serde_as]
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct BybitHttpResponse<T> {
    #[serde(rename = "retCode")]
    pub ret_code: u64,

    #[serde(rename = "retMsg")]
    pub ret_msg: String,

    #[serde_as(as = "TimestampMilliSeconds")]
    #[serde(rename = "time")]
    pub time: DateTime<Utc>,

    #[serde(rename = "result")]
    pub result: T,
}

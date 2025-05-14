use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{DefaultOnError, TimestampMilliSeconds, serde_as};

use super::types::InstrumentCategory;

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

/// Generic response from Bybit used for the list of results.
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct ResultList<T> {
    #[serde(rename = "list")]
    pub list: Vec<T>,

    #[serde_as(deserialize_as = "DefaultOnError")]
    #[serde(default, rename = "nextPageCursor")]
    pub next_page_cursor: Option<String>,

    #[serde(rename = "category")]
    pub category: Option<InstrumentCategory>,
}

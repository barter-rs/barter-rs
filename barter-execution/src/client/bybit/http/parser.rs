use barter_integration::protocol::http::HttpParser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampMilliSeconds};

use crate::error::{ApiError, UnindexedClientError};

#[derive(Debug, Clone)]
pub struct BybitParser;

impl HttpParser for BybitParser {
    type ApiError = ByBitHttpApiError;
    type OutputError = UnindexedClientError;

    fn parse_api_error(
        &self,
        status: reqwest::StatusCode,
        error: Self::ApiError,
    ) -> Self::OutputError {
        let api_error = if status.is_success() {
            match error.ret_code {
                10007 => ApiError::Unauthorized(error.ret_msg),
                10006 | 10018 => ApiError::RateLimit,
                110001 => ApiError::OrderNotFound,
                _ => ApiError::Custom(serde_json::to_string(&error).unwrap()),
            }
        } else {
            ApiError::Custom(error.ret_msg)
        };

        UnindexedClientError::Api(api_error)
    }
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct ByBitHttpApiError {
    #[serde(rename = "retCode")]
    pub ret_code: u64,

    #[serde(rename = "retMsg")]
    pub ret_msg: String,

    #[serde_as(as = "TimestampMilliSeconds")]
    #[serde(rename = "time")]
    pub time: DateTime<Utc>,
}

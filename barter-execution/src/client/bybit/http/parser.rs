use barter_integration::protocol::http::HttpParser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{TimestampMilliSeconds, serde_as};

use crate::error::{UnindexedApiError, UnindexedClientError};

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
                10007 => UnindexedApiError::Unauthorized(error.ret_msg),
                10006 | 10018 => UnindexedApiError::RateLimit,
                110001 => UnindexedApiError::OrderNotFound,
                _ => UnindexedApiError::Custom(serde_json::to_string(&error).unwrap()),
            }
        } else {
            UnindexedApiError::Custom(error.ret_msg)
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

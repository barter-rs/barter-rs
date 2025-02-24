use crate::error::UnindexedClientError;
use barter_integration::protocol::http::HttpParser;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct BinanceSpotHttpParser;

impl HttpParser for BinanceSpotHttpParser {
    type ApiError = BinanceHttpApiError;
    type OutputError = UnindexedClientError;

    fn parse_api_error(
        &self,
        status: reqwest::StatusCode,
        error: Self::ApiError,
    ) -> Self::OutputError {
        todo!()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceHttpApiError;

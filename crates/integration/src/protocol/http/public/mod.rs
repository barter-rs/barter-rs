use super::BuildStrategy;
use crate::error::SocketError;

/// [`RestRequest`](super::RestRequest) [`BuildStrategy`] that builds a non-authenticated Http request with no headers.
#[derive(Debug, Copy, Clone)]
pub struct PublicNoHeaders;

impl BuildStrategy for PublicNoHeaders {
    fn build<Request>(
        &self,
        _: Request,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Request, SocketError> {
        builder.build().map_err(SocketError::from)
    }
}

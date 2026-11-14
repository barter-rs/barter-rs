use self::rest::RestRequest;
use crate::error::SocketError;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use tracing::error;

/// Defines an abstract [`RestRequest`] that can be executed by a fully
/// configurable [`RestClient`](rest::client::RestClient).
pub mod rest;

/// Defines a configurable [`RequestSigner`](private::RequestSigner) that signs Http
/// [`RestRequest`] using API specific logic.
pub mod private;

/// Defines a default [`BuildStrategy`] that builds a non-authenticated Http
/// [`RestRequest`] with no headers.
pub mod public;

/// [`RestRequest`] build strategy for the API being interacted with.
///
/// An API that requires authenticated [`RestRequest`]s will likely utilise the configurable
/// [`RequestSigner`](private::RequestSigner) to sign the requests before building.
///
/// An API that requires no authentication may just add mandatory `reqwest` headers to the
/// [`RestRequest`] before building.
pub trait BuildStrategy {
    /// Use a [`RestRequest`] and [`reqwest::RequestBuilder`] to construct a [`reqwest::Request`]
    /// that is ready for executing.
    ///
    /// It is expected that any signing or performed during this method, or the addition of any
    /// `reqwest` headers.
    fn build<Request>(
        &self,
        request: Request,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Request, SocketError>
    where
        Request: RestRequest;
}

/// Utilised by a [`RestClient`](rest::client::RestClient) to deserialise
/// [`RestRequest::Response`], and upon failure parses API errors
/// returned from the server.
pub trait HttpParser {
    type ApiError: DeserializeOwned;
    type OutputError: From<SocketError>;

    /// Attempt to parse a [`StatusCode`] & bytes payload into a deserialisable `Response`.
    fn parse<Response>(
        &self,
        status: StatusCode,
        payload: &[u8],
    ) -> Result<Response, Self::OutputError>
    where
        Response: DeserializeOwned,
    {
        // Attempt to deserialise reqwest::Response bytes into Ok(Response)
        let parse_ok_error = match serde_json::from_slice::<Response>(payload) {
            Ok(response) => return Ok(response),
            Err(serde_error) => serde_error,
        };

        // Attempt to deserialise API Error if Ok(Response) deserialisation failed
        let parse_api_error_error = match serde_json::from_slice::<Self::ApiError>(payload) {
            Ok(api_error) => return Err(self.parse_api_error(status, api_error)),
            Err(serde_error) => serde_error,
        };

        // Log errors if failed to deserialise reqwest::Response into Response or API Self::Error
        error!(
            status_code = ?status,
            ?parse_ok_error,
            ?parse_api_error_error,
            response_body = %String::from_utf8_lossy(payload),
            "error deserializing HTTP response"
        );

        Err(Self::OutputError::from(SocketError::DeserialiseBinary {
            error: parse_ok_error,
            payload: payload.to_vec(),
        }))
    }

    /// If [`parse`](Self::parse) fails to deserialise the `Ok(Response)`, this function parses
    /// to parse the API [`Self::ApiError`] associated with the response.
    fn parse_api_error(&self, status: StatusCode, error: Self::ApiError) -> Self::OutputError;
}

use crate::{
    exchange::Connector,
    instrument::InstrumentData,
    subscription::{Map, SubscriptionKind},
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    protocol::{
        websocket::{WebSocket, WebSocketParser},
        StreamParser,
    },
    Validator,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Defines how to validate that actioned market data
/// [`Subscription`](crate::subscription::Subscription)s were accepted by the exchange.
#[async_trait]
pub trait SubscriptionValidator {
    type Parser: StreamParser;

    async fn validate<Exchange, Instrument, Kind>(
        instrument_map: Map<Instrument::Id>,
        websocket: &mut WebSocket,
    ) -> Result<Map<Instrument::Id>, SocketError>
    where
        Exchange: Connector + Send,
        Instrument: InstrumentData,
        Kind: SubscriptionKind + Send;
}

/// Standard [`SubscriptionValidator`] for [`WebSocket`]s suitable for most exchanges.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct WebSocketSubValidator;

#[async_trait]
impl SubscriptionValidator for WebSocketSubValidator {
    type Parser = WebSocketParser;

    async fn validate<Exchange, Instrument, Kind>(
        instrument_map: Map<Instrument::Id>,
        websocket: &mut WebSocket,
    ) -> Result<Map<Instrument::Id>, SocketError>
    where
        Exchange: Connector + Send,
        Instrument: InstrumentData,
        Kind: SubscriptionKind + Send,
    {
        // Establish exchange specific subscription validation parameters
        let timeout = Exchange::subscription_timeout();
        let expected_responses = Exchange::expected_responses(&instrument_map);

        // Parameter to keep track of successful Subscription outcomes
        let mut success_responses = 0usize;

        loop {
            // Break if all Subscriptions were a success
            if success_responses == expected_responses {
                debug!(exchange = %Exchange::ID, "validated exchange WebSocket subscriptions");
                break Ok(instrument_map);
            }

            tokio::select! {
                // If timeout reached, return SubscribeError
                _ = tokio::time::sleep(timeout) => {
                    break Err(SocketError::Subscribe(
                        format!("subscription validation timeout reached: {:?}", timeout)
                    ))
                },
                // Parse incoming messages and determine subscription outcomes
                message = websocket.next() => {
                    let response = match message {
                        Some(response) => response,
                        None => break Err(SocketError::Subscribe("WebSocket stream terminated unexpectedly".to_string()))
                    };

                    match Self::Parser::parse::<Exchange::SubResponse>(response) {
                        Some(Ok(response)) => match response.validate() {
                            // Subscription success
                            Ok(response) => {
                                success_responses += 1;
                                debug!(
                                    exchange = %Exchange::ID,
                                    %success_responses,
                                    %expected_responses,
                                    payload = ?response,
                                    "received valid Ok subscription response",
                                );
                            }

                            // Subscription failure
                            Err(err) => break Err(err)
                        }
                        Some(Err(SocketError::Deserialise { error, payload })) if success_responses >= 1 => {
                            // Already active subscription payloads, so skip to next SubResponse
                            debug!(
                                exchange = %Exchange::ID,
                                ?error,
                                %success_responses,
                                %expected_responses,
                                %payload,
                                "failed to deserialise non SubResponse payload"
                            );
                            continue
                        }
                        Some(Err(SocketError::Terminated(close_frame))) => {
                            break Err(SocketError::Subscribe(
                                format!("received WebSocket CloseFrame: {close_frame}")
                            ))
                        }
                        _ => {
                            // Pings, Pongs, Frames, etc.
                            continue
                        }
                    }
                }
            }
        }
    }
}

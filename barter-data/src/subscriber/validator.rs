use crate::{
    exchange::Connector,
    subscription::{Map, SubscriptionKind},
};
use async_trait::async_trait;
use barter_integration::{
    Validator,
    error::SocketError,
    protocol::{
        StreamParser,
        websocket::{WebSocket, WebSocketSerdeParser, WsMessage},
    },
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Defines how to validate that actioned market data
/// [`Subscription`](crate::subscription::Subscription)s were accepted by the exchange.
#[async_trait]
pub trait SubscriptionValidator {
    type Parser;

    async fn validate<Exchange, InstrumentKey, Kind>(
        instrument_map: Map<InstrumentKey>,
        websocket: &mut WebSocket,
    ) -> Result<(Map<InstrumentKey>, Vec<WsMessage>), SocketError>
    where
        Exchange: Connector + Send,
        InstrumentKey: Send,
        Kind: SubscriptionKind + Send;
}

/// Standard [`SubscriptionValidator`] for [`WebSocket`]s suitable for most exchanges.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct WebSocketSubValidator;

#[async_trait]
impl SubscriptionValidator for WebSocketSubValidator {
    type Parser = WebSocketSerdeParser;

    async fn validate<Exchange, Instrument, Kind>(
        instrument_map: Map<Instrument>,
        websocket: &mut WebSocket,
    ) -> Result<(Map<Instrument>, Vec<WsMessage>), SocketError>
    where
        Exchange: Connector + Send,
        Instrument: Send,
        Kind: SubscriptionKind + Send,
    {
        // Establish exchange specific subscription validation parameters
        let timeout = Exchange::subscription_timeout();
        let expected_responses = Exchange::expected_responses(&instrument_map);

        // Parameter to keep track of successful Subscription outcomes
        let mut success_responses = 0usize;

        // Buffer any active Subscription market events that are received during validation
        let mut buff_active_subscription_events = Vec::new();

        loop {
            // Break if all Subscriptions were a success
            if success_responses == expected_responses {
                debug!(exchange = %Exchange::ID, "validated exchange WebSocket subscriptions");
                break Ok((instrument_map, buff_active_subscription_events));
            }

            tokio::select! {
                // If timeout reached, return SubscribeError
                _ = tokio::time::sleep(timeout) => {
                    break Err(SocketError::Subscribe(
                        format!("subscription validation timeout reached: {timeout:?}")
                    ))
                },
                // Parse incoming messages and determine subscription outcomes
                message = websocket.next() => {
                    let response = match message {
                        Some(response) => response,
                        None => break Err(SocketError::Subscribe("WebSocket stream terminated unexpectedly".to_string()))
                    };

                    match <WebSocketSerdeParser as StreamParser<Exchange::SubResponse>>::parse(response) {
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
                        Some(Err(SocketError::Deserialise { error: _, payload })) => {
                            // Most likely already active subscription payload, so add to market
                            // event buffer for post validation processing
                            buff_active_subscription_events.push(WsMessage::text(payload));
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

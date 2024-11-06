use super::subscription::IbkrPlatformEvent;
use crate::{
    exchange::{ibkr::subscription::IbkrSubResponse, Connector, ExchangeSub},
    subscriber::validator::SubscriptionValidator,
    subscription::{Map, SubscriptionKind},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    subscription::SubscriptionId,
    protocol::{
        websocket::{WebSocket, WebSocketParser, WsMessage},
        StreamParser,
    },
    Validator,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use smol_str::ToSmolStr;
use tracing::{debug, error, warn};

/// [`Ibkr`](super::Ibkr) specific [`SubscriptionValidator`].
///
/// ### Notes
/// - Required because Ibkr has a series of messages to receive before subscriptions can be
///   validated.
/// - The [`IbkrChannelId`](super::subscription::IbkrChannelId) is used to identify the
///   [`Subscription`](crate::subscription::Subscription) associated with incoming
///   events, rather than a `String` channel-market identifier.
/// - Therefore the [`SubscriptionId`] format must change during [`IbkrWebSocketSubValidator::validate`]
///   to use the [`IbkrChannelId`](super::subscription::IbkrChannelId)
///   (see module level "SubscriptionId" documentation notes for more details).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct IbkrWebSocketSubValidator;

#[async_trait]
impl SubscriptionValidator for IbkrWebSocketSubValidator {
    type Parser = WebSocketParser;

    async fn validate<Exchange, InstrumentKey, Kind>(
        mut instrument_map: Map<InstrumentKey>,
        websocket: &mut WebSocket,
    ) -> Result<(Map<InstrumentKey>, Vec<WsMessage>), SocketError>
    where
        Exchange: Connector + Send,
        InstrumentKey: Send,
        Kind: SubscriptionKind + Send,
    {
        // Establish exchange specific subscription validation parameters
        let timeout = Exchange::subscription_timeout();
        let expected_responses = Exchange::expected_responses(&instrument_map);

        // Parameter to keep track of successful Subscription outcomes
        // '--> Ibkr sends system, status, and account as the first messages, so count them also
        let mut success_responses = 0usize;

        // Buffer any active Subscription market events that are received during validation
        let mut buff_active_subscription_events = Vec::new();

        loop {
            // debug!(exchange = %Exchange::ID, success_responses, init_snapshots_received, "validating exchange WebSocket subscriptions");
            // Break if all Subscriptions were a success
            if success_responses == expected_responses
                // && init_snapshots_received == expected_responses
            {
                debug!(exchange = %Exchange::ID, "validated exchange WebSocket subscriptions");
                break Ok((instrument_map, buff_active_subscription_events));
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
                        Some(r) => r,
                        None => break Err(SocketError::Subscribe("WebSocket stream terminated unexpectedly".to_string()))
                    };

                    match Self::Parser::parse::<IbkrPlatformEvent>(response) {
                        Some(Ok(resp)) => match resp.validate() {
                            // Ibkr server system message
                            Ok(IbkrPlatformEvent::System(system)) => {
                                if system.username.is_some() {
                                    success_responses += 1;
                                }
                                // debug!(
                                //     exchange = %Exchange::ID,
                                //     %success_responses,
                                //     %expected_responses,
                                //     payload = ?system,
                                //     "received Ibkr system message",
                                // );
                            }

                            // Authentication status message
                            Ok(IbkrPlatformEvent::AuthnStatus(status)) => {
                                if status.args.authenticated == true {
                                    success_responses += 1;
                                }
                                // debug!(
                                //     exchange = %Exchange::ID,
                                //     %success_responses,
                                //     %expected_responses,
                                //     payload = ?status,
                                //     "received Ibkr authentication status message",
                                // );
                            }

                            // Account Update message
                            Ok(IbkrPlatformEvent::Account(account)) => {
                                if !account.args.selected_account.is_empty() {
                                    success_responses += 1;
                                }
                                // debug!(
                                //     exchange = %Exchange::ID,
                                //     %success_responses,
                                //     %expected_responses,
                                //     payload = ?account,
                                //     "received Ibkr account update message",
                                // );
                            }

                            // Subscription success
                            Ok(IbkrPlatformEvent::Subscribed(subresp)) => {
                                // Determine SubscriptionId associated with the success response
                                let IbkrSubResponse { channel, market, channel_id } = &subresp;
                                let subscription_id = ExchangeSub::from((channel, market)).id();

                                // Replace SubscriptionId with SubscriptionId(channel_id)
                                if let Some(subscription) = instrument_map.0.remove(&subscription_id) {
                                    success_responses += 1;
                                    instrument_map.0.insert(SubscriptionId(channel_id.0.to_smolstr()), subscription);

                                    // debug!(
                                    //     exchange = %Exchange::ID,
                                    //     %success_responses,
                                    //     %expected_responses,
                                    //     payload = ?subresp,
                                    //     ?subscription_id,
                                    //     // ?instrument_map,
                                    //     "received valid Ok subscription response",
                                    // );

                                    // init_snapshots_received += 1;
                                    // buff_active_subscription_events.push(response.clone().unwrap());

                                }
                            }

                            // Subscription failure
                            Err(err) => break Err(err),

                            // Not reachable after IbkrPlatformEvent validate()
                            Ok(IbkrPlatformEvent::Error(error)) => panic!("{error:?}"),
                        }
                        Some(Err(SocketError::Deserialise { error: _, payload })) if success_responses >= 1 => {
                            // TODO: this is copy pasta, true?
                            // Already active Ibkr subscriptions will send initial snapshots
                            warn!(?payload, "init_snapshots_received in Deserialise Error");
                            buff_active_subscription_events.push(WsMessage::Text(payload));
                            continue
                        }
                        Some(Err(SocketError::Terminated(close_frame))) => {
                            break Err(SocketError::Subscribe(
                                format!("received WebSocket CloseFrame: {close_frame}")
                            ))
                        }
                        resp => {
                            // Pings, Pongs, Frames, etc.
                            error!("received unhandled WebSocket response: {resp:?}");
                            continue
                        }
                    }
                }
            }
        }
    }
}

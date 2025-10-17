use super::subscription::{BitfinexPlatformEvent, BitfinexSubResponse};
use crate::{
    Identifier,
    exchange::{Connector, ExchangeSub},
    subscriber::validator::SubscriptionValidator,
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
    subscription::SubscriptionId,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use smol_str::ToSmolStr;
use tracing::debug;

/// [`Bitfinex`](super::Bitfinex) specific [`SubscriptionValidator`].
///
/// ### Notes
/// - Required because Bitfinex has a non-self-describing data format after subscriptions have been
///   validated.
/// - The [`BitfinexChannelId`](super::subscription::BitfinexChannelId) is used to identify the
///   [`Subscription`](crate::subscription::Subscription) associated with incoming
///   events, rather than a `String` channel-market identifier.
/// - Therefore the [`SubscriptionId`] format must change during [`BitfinexWebSocketSubValidator::validate`]
///   to use the [`BitfinexChannelId`](super::subscription::BitfinexChannelId)
///   (see module level "SubscriptionId" documentation notes for more details).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexWebSocketSubValidator;

#[async_trait]
impl SubscriptionValidator for BitfinexWebSocketSubValidator {
    type Parser = WebSocketSerdeParser;

    async fn validate<Exchange, Instrument, Kind>(
        mut instrument_map: Map<Instrument>,
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
        // '--> Bitfinex sends snapshots as the first message, so count them also
        let mut success_responses = 0usize;
        let mut init_snapshots_received = 0usize;

        // Buffer any active Subscription market events that are received during validation
        let mut buff_active_subscription_events = Vec::new();

        loop {
            // Break if all Subscriptions were a success
            if success_responses == expected_responses
                && init_snapshots_received == expected_responses
            {
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

                    match <WebSocketSerdeParser as StreamParser<BitfinexPlatformEvent>>::parse(response) {
                        Some(Ok(response)) => match response.validate() {
                            // Bitfinex server is online
                            Ok(BitfinexPlatformEvent::PlatformStatus(status)) => {
                                debug!(
                                    exchange = %Exchange::ID,
                                    %success_responses,
                                    %expected_responses,
                                    payload = ?status,
                                    "received Bitfinex platform status",
                                );
                            }

                            // Subscription success
                            Ok(BitfinexPlatformEvent::Subscribed(response)) => {
                                // Determine SubscriptionId associated with the success response
                                let BitfinexSubResponse { channel, market, channel_id } = &response;
                                let subscription_id = ExchangeSub::from((channel, market)).id();

                                // Replace SubscriptionId with SubscriptionId(channel_id)
                                if let Some(subscription) = instrument_map.0.remove(&subscription_id) {
                                    success_responses += 1;
                                    instrument_map.0.insert(SubscriptionId(channel_id.0.to_smolstr()), subscription);

                                    debug!(
                                        exchange = %Exchange::ID,
                                        %success_responses,
                                        %expected_responses,
                                        payload = ?response,
                                        "received valid Ok subscription response",
                                    );
                                }
                            }

                            // Subscription failure
                            Err(err) => break Err(err),

                            // Not reachable after BitfinexPlatformEvent validate()
                            Ok(BitfinexPlatformEvent::Error(error)) => panic!("{error:?}"),
                        }
                        Some(Err(SocketError::Deserialise { error: _, payload })) if success_responses >= 1 => {
                            // Already active Bitfinex subscriptions will send initial snapshots
                            init_snapshots_received += 1;
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

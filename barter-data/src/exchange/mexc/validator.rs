use super::subscription::MexcSubResponse;
use crate::{
    exchange::Connector,
    subscriber::validator::SubscriptionValidator,
    subscription::{Map, SubscriptionKind},
};
use async_trait::async_trait;
use barter_integration::{
    Validator,
    error::SocketError,
    protocol::{
        StreamParser,
        websocket::{WebSocket, WebSocketParser, WsMessage},
    },
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// `SubscriptionValidator` for MEXC that parses JSON confirmations while
/// buffering any binary frames until validation completes.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct MexcWebSocketSubValidator;

#[async_trait]
impl SubscriptionValidator for MexcWebSocketSubValidator {
    type Parser = WebSocketParser;

    async fn validate<Exchange, Instrument, Kind>(
        instrument_map: Map<Instrument>,
        websocket: &mut WebSocket,
    ) -> Result<(Map<Instrument>, Vec<WsMessage>), SocketError>
    where
        Exchange: Connector + Send,
        Instrument: Send,
        Kind: SubscriptionKind + Send,
    {
        let timeout = Exchange::subscription_timeout();
        let expected_responses = Exchange::expected_responses(&instrument_map);
        let mut success_responses = 0usize;
        let mut buff_active_subscription_events = Vec::new();

        loop {
            if success_responses >= expected_responses {
                debug!(exchange = %Exchange::ID, "validated MEXC WebSocket subscriptions");
                break Ok((instrument_map, buff_active_subscription_events));
            }

            tokio::select! {
                _ = tokio::time::sleep(timeout) => {
                    break Err(SocketError::Subscribe(
                        format!("subscription validation timeout reached: {timeout:?}")
                    ))
                },
                maybe_message = websocket.next() => {
                    let response = match maybe_message {
                        Some(r) => r,
                        None => break Err(SocketError::Subscribe("WebSocket stream terminated unexpectedly".to_string()))
                    };

                    match response {
                        Ok(ref ws_msg) => {
                            match <WebSocketParser as StreamParser<MexcSubResponse>>::parse(Ok(ws_msg.clone())) {
                                Some(Ok(sub)) => match sub.validate() {
                                    Ok(validated) => {
                                        success_responses += 1;
                                        debug!(exchange = %Exchange::ID, %success_responses, %expected_responses, payload = ?validated, "received valid Ok subscription response");
                                    }
                                    Err(err) => break Err(err),
                                },
                                Some(Err(SocketError::Terminated(close_frame))) => {
                                    break Err(SocketError::Subscribe(
                                        format!("received WebSocket CloseFrame: {close_frame}")
                                    ))
                                }
                                _ => {
                                    buff_active_subscription_events.push(ws_msg.clone());
                                    continue;
                                }
                            }
                        }
                        Err(err) => return Err(SocketError::WebSocket(Box::new(err))),
                    }
                }
            }
        }
    }
}

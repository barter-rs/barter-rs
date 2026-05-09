//! Protocol-layer combinators for a single WebSocket connection.
//!
//! Operates between the raw connection and `with_serde`:
//!   `Sink<WsMessage> + Stream<Item = Message<AdminWs, Bytes>>`
//!
//! Each piece has one job (SRP), composed at the call site.

use crate::{
    Message,
    protocol::websocket::{AdminWs, WsError, WsMessage, is_websocket_disconnected},
};
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt, stream::select};
use std::time::Duration;
use tokio::time::{Instant, interval};
use tokio_stream::wrappers::IntervalStream;
use tracing::{debug, warn};


/// Action signalled by a protocol-layer event.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProtocolAction {
    Continue,
    Terminate,
}

/// Classify a [`WsError`] as fatal (terminate the socket) or transient (continue).
///
/// SRP: pure decision function. No side effects, no state.
pub fn classify_ws_error(error: &WsError) -> ProtocolAction {
    if is_websocket_disconnected(error) {
        ProtocolAction::Terminate
    } else {
        ProtocolAction::Continue
    }
}

/// Outbound heartbeat: emits a `WsMessage::Ping` on every tick of `period`.
///
/// SRP: drives the Ping schedule. Does not own the sink, does not track Pongs.
/// Caller merges this stream into whatever drives the sink.
pub fn heartbeat_pings(period: Duration) -> impl Stream<Item = WsMessage> {
    IntervalStream::new(interval(period)).map(|_| WsMessage::Ping(Bytes::new()))
}

/// Tracks outstanding outbound Pings and reports a timeout if no Pong arrives.
///
/// SRP: liveness state only. Does not drive Pings, does not touch the socket.
#[derive(Debug)]
pub struct PongTracker {
    timeout: Duration,
    last_ping_at: Option<Instant>,
}

impl PongTracker {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            last_ping_at: None,
        }
    }

    /// Record that a Ping was sent.
    pub fn record_ping_sent(&mut self) {
        self.last_ping_at = Some(Instant::now());
    }

    /// Record that a Pong was received — clears any outstanding deadline.
    pub fn record_pong_received(&mut self) {
        self.last_ping_at = None;
    }

    /// Check whether the outstanding Ping (if any) has exceeded the timeout.
    pub fn check(&self) -> ProtocolAction {
        match self.last_ping_at {
            Some(t) if t.elapsed() >= self.timeout => ProtocolAction::Terminate,
            _ => ProtocolAction::Continue,
        }
    }
}

/// Composition example: drive one socket with heartbeat-out, pong-timeout, and
/// graceful close on fatal events. Forwards inbound payloads via `on_payload`.
///
/// Returns when the socket closes, errors fatally, or the user `to_send` ends.
pub async fn run_protocol_layer<S, ToSend, OnPayload>(
    mut socket: S,
    to_send: ToSend,
    heartbeat_period: Duration,
    pong_timeout: Duration,
    mut on_payload: OnPayload,
) -> Result<(), WsError>
where
    S: Sink<WsMessage, Error = WsError> + Stream<Item = Message<AdminWs, Bytes>> + Unpin,
    ToSend: Stream<Item = WsMessage> + Unpin,
    OnPayload: FnMut(Bytes),
{
    let mut sink_input = select(to_send, heartbeat_pings(heartbeat_period));
    let mut pong = PongTracker::new(pong_timeout);
    let mut pong_check = interval(pong_timeout / 2);

    loop {
        tokio::select! {
            Some(out) = sink_input.next() => {
                if matches!(out, WsMessage::Ping(_)) {
                    pong.record_ping_sent();
                }
                socket.send(out).await?;
            }
            Some(inbound) = socket.next() => match inbound {
                Message::Payload(bytes) => on_payload(bytes),
                Message::Admin(AdminWs::Pong(_)) => pong.record_pong_received(),
                Message::Admin(AdminWs::Ping(_)) => {} // tungstenite auto-pongs
                Message::Admin(AdminWs::Close(frame)) => {
                    debug!(?frame, "peer sent Close — terminating");
                    break;
                }
                Message::Admin(AdminWs::WsError(error)) => {
                    if classify_ws_error(&error) == ProtocolAction::Terminate {
                        warn!(?error, "fatal WebSocket error — terminating");
                        return Err(error);
                    }
                    debug!(?error, "transient WebSocket error — continuing");
                }
            },
            _ = pong_check.tick() => {
                if pong.check() == ProtocolAction::Terminate {
                    warn!(?pong_timeout, "pong timeout exceeded — terminating");
                    break;
                }
            }
            else => break,
        }
    }

    socket.close().await
}

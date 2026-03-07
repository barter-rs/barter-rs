use std::{
    collections::HashSet,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use barter_instrument::{
    asset::name::AssetNameExchange, exchange::ExchangeId, instrument::name::InstrumentNameExchange,
};
use barter_integration::{
    Transformer,
    error::SocketError,
    protocol::{
        http::private::encoder::{Encoder, HexEncoder},
        websocket::{WebSocket, WebSocketParser, WsMessage, WsSink, WsStream},
    },
    stream::ExchangeStream,
};
use chrono::{Duration, Utc};
use derive_more::Constructor;
use futures::{SinkExt, Stream, StreamExt};
use hmac::{Hmac, Mac};
use payload::{BybitPayload, BybitPayloadTopic, OrderExecutionData, OrderUpdateData};
use pin_project::pin_project;
use sha2::Sha256;
use tokio::{sync::mpsc, time::timeout};
use tracing::{debug, error, warn};

use super::servers::BybitServer;
use crate::{
    AccountEvent, AccountEventKind, AccountSnapshot, ApiCredentials, UnindexedAccountEvent,
    error::{ConnectivityError, UnindexedClientError},
};

mod payload;
mod subscription;

/// Bybit account related event stream
#[derive(Debug, Constructor)]
#[pin_project]
pub struct BybitAccountStream<Server>
where
    Server: BybitServer,
{
    #[pin]
    inner: ExchangeStream<WebSocketParser, WsStream, BybitAccountStreamTransformer<Server>>,
}

impl<Server> Stream for BybitAccountStream<Server>
where
    Server: BybitServer,
{
    type Item = UnindexedAccountEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().inner.poll_next(cx) {
            Poll::Ready(Some(item)) => match item {
                Ok(item) => Poll::Ready(Some(item)),
                Err(err) => match err {
                    // We are ignoring `Deserialise` errors. This happens in
                    // case of pong messages because the system doesn't know how
                    // to deserialize them.
                    SocketError::Deserialise { .. } => {
                        warn!(?err, "error received from the BybitAccountStream");

                        // Wake to start polling the future again
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    // We received an error from the underlying stream. Log an error
                    // and indicate exhaustion on the current stream.
                    _ => {
                        error!(?err, "Terminal error received. Closing the stream.");
                        Poll::Ready(None)
                    }
                },
            },
            // Underlying stream finished
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug)]
pub struct BybitAccountStreamTransformer<Server> {
    assets: HashSet<AssetNameExchange>,
    instruments: HashSet<InstrumentNameExchange>,
    _phantom: PhantomData<Server>,
}

impl<Server> BybitAccountStreamTransformer<Server> {
    pub fn new<A, I>(assets: A, instruments: I) -> Self
    where
        A: IntoIterator<Item = AssetNameExchange>,
        I: IntoIterator<Item = InstrumentNameExchange>,
    {
        Self {
            assets: assets.into_iter().collect(),
            instruments: instruments.into_iter().collect(),
            _phantom: PhantomData,
        }
    }

    fn filter_event(&self, event: UnindexedAccountEvent) -> Option<UnindexedAccountEvent> {
        match &event.kind {
            AccountEventKind::Snapshot(snapshot) => self.filter_account_snapshot(snapshot),
            AccountEventKind::BalanceSnapshot(snapshot) => {
                let asset = &snapshot.value().asset;
                self.allowed_asset(asset).then_some(event)
            }
            AccountEventKind::OrderSnapshot(snapshot) => {
                let instrument = &snapshot.value().key.instrument;
                self.allowed_instrument(instrument).then_some(event)
            }
            AccountEventKind::OrderCancelled(order_event) => {
                let instrument = &order_event.key.instrument;
                self.allowed_instrument(instrument).then_some(event)
            }
            AccountEventKind::Trade(trade) => {
                let instrument = &trade.instrument;
                self.allowed_instrument(instrument).then_some(event)
            }
        }
    }

    fn filter_account_snapshot(
        &self,
        snapshot: &AccountSnapshot<ExchangeId, AssetNameExchange, InstrumentNameExchange>,
    ) -> Option<UnindexedAccountEvent> {
        let balances = snapshot
            .balances
            .iter()
            .filter(|balance| self.allowed_asset(&balance.asset))
            .cloned()
            .collect::<Vec<_>>();

        let instruments = snapshot
            .instruments
            .iter()
            .filter(|instrument| self.allowed_instrument(&instrument.instrument))
            .cloned()
            .collect::<Vec<_>>();

        if balances.is_empty() && instruments.is_empty() {
            debug!(
                ?snapshot,
                "No instruments or assets supported from the received snapshot"
            );
            return None;
        }

        Some(AccountEvent::new(
            snapshot.exchange,
            AccountEventKind::Snapshot(AccountSnapshot::new(
                snapshot.exchange,
                balances,
                instruments,
            )),
        ))
    }

    fn allowed_asset(&self, asset: &AssetNameExchange) -> bool {
        self.assets.contains(asset)
    }

    fn allowed_instrument(&self, instrument: &InstrumentNameExchange) -> bool {
        self.instruments.contains(instrument)
    }
}

impl<Server> Transformer for BybitAccountStreamTransformer<Server>
where
    Server: BybitServer,
{
    type Error = SocketError;
    type Input = BybitPayload;
    type Output = UnindexedAccountEvent;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, message: Self::Input) -> Self::OutputIter {
        let events = match parse_message::<Server>(message) {
            Ok(events) => events,
            Err(err) => return vec![Err(err)],
        };

        events
            .into_iter()
            .filter_map(|event| match event {
                // In this invocation, we are removing unsupported assets and
                // instruments from the event and wrapping it in `Ok`, if it
                // still exists after it was filtered
                Ok(event) => self.filter_event(event).and_then(|event| Some(Ok(event))),
                // We keep the errors. They are handled later
                Err(err) => Some(Err(err)),
            })
            .collect()
    }
}

fn parse_message<Server>(
    message: BybitPayload,
) -> Result<Vec<Result<UnindexedAccountEvent, SocketError>>, SocketError>
where
    Server: BybitServer,
{
    let events = match message.topic {
        BybitPayloadTopic::Order => {
            serde_json::from_str::<Vec<OrderUpdateData>>(message.data.get())
                .map_err(|err| SocketError::Deserialise {
                    error: err,
                    payload: message.data.to_string(),
                })?
                .into_iter()
                .map(|data| UnindexedAccountEvent::try_from((Server::ID, data, message.timestamp)))
                .collect()
        }
        BybitPayloadTopic::Execution => {
            serde_json::from_str::<Vec<OrderExecutionData>>(message.data.get())
                .map_err(|err| SocketError::Deserialise {
                    error: err,
                    payload: message.data.to_string(),
                })?
                .into_iter()
                .map(|data| UnindexedAccountEvent::try_from((Server::ID, data, message.timestamp)))
                .collect()
        }
    };

    Ok(events)
}

/// Sends the message over the websocket and validates if the response indicates
/// a successful action. If the response is not received the validation timeout.
pub async fn send_validate(
    websocket: &mut WebSocket,
    message: WsMessage,
) -> Result<(), UnindexedClientError> {
    // Send the message
    websocket.send(message.clone()).await?;
    debug!(payload = ?message, "Websocket message sent");

    // Receive response message
    let timeout_duration = std::time::Duration::from_secs(5);
    let response = timeout(timeout_duration, websocket.next())
        .await
        .map_err(|_| UnindexedClientError::Connectivity(ConnectivityError::Timeout))?;

    // Validate response message
    if let Some(Ok(payload)) = response {
        debug!(?payload, "Received response");
        let text = payload.to_text().map_err(|_| {
            UnindexedClientError::Connectivity(ConnectivityError::Socket("".to_string()))
        })?;

        if text.contains(r#""success":true"#) {
            return Ok(());
        }
    }

    Err(UnindexedClientError::Connectivity(
        ConnectivityError::Socket(format!("Websocket message not confirmed: {message}")),
    ))
}

pub fn generate_auth_message(credentials: &ApiCredentials) -> WsMessage {
    let expires_at = (Utc::now() + Duration::seconds(5)).timestamp_millis();
    let message = format!("GET/realtime{}", expires_at);
    let signature = {
        let mut signed_key = Hmac::<Sha256>::new_from_slice(credentials.secret.as_bytes())
            .expect("secret should have a valid length");
        signed_key.update(message.as_bytes());
        HexEncoder.encode(signed_key.finalize().into_bytes())
    };

    WsMessage::text(
        serde_json::json!({
            "op": "auth",
            "args": [
                credentials.key,
                expires_at,
                signature,
            ]
        })
        .to_string(),
    )
}

pub fn generate_subscription_message() -> WsMessage {
    WsMessage::text(
        serde_json::json!({
            "op": "subscribe",
            // TODO: Use faster
            // https://bybit-exchange.github.io/docs/v5/websocket/private/fast-execution
            // for the normal trades. For funding fees and other more exotic
            // trades we can still use the slower channel.
            "args": ["order", "execution"]
        })
        .to_string(),
    )
}

//
// TODO: The code bellow is copied from the `barter-data` as it is (without docs).
// Can we move it to `barter-integration` to be used here?
//

pub async fn distribute_messages_to_exchange(
    exchange: ExchangeId,
    mut ws_sink: WsSink,
    mut ws_sink_rx: mpsc::UnboundedReceiver<WsMessage>,
) {
    while let Some(message) = ws_sink_rx.recv().await {
        if let Err(error) = ws_sink.send(message).await {
            if barter_integration::protocol::websocket::is_websocket_disconnected(&error) {
                break;
            }

            // Log error only if WsMessage failed to send over a connected WebSocket
            error!(
                %exchange,
                %error,
                "failed to send output message to the exchange via WsSink"
            );
        }
    }
}

#[derive(Debug)]
pub struct PingInterval {
    pub interval: tokio::time::Interval,
    pub ping: fn() -> WsMessage,
}

pub async fn schedule_pings_to_exchange(
    exchange: ExchangeId,
    ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
    PingInterval { mut interval, ping }: PingInterval,
) {
    loop {
        // Wait for next scheduled ping
        interval.tick().await;

        // Construct exchange custom application-level ping payload
        let payload = ping();
        debug!(%exchange, %payload, "sending custom application-level ping to exchange");

        if ws_sink_tx.send(payload).is_err() {
            break;
        }
    }
}

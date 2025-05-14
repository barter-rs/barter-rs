use std::{
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
use tokio::sync::mpsc;
use tracing::warn;
use tracing::{debug, error};

use crate::{ApiCredentials, UnindexedAccountEvent, error::UnindexedClientError};

use super::servers::BybitServer;

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
                        error!(?err, "error received from the BybitAccountStream");
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
    assets: Vec<AssetNameExchange>,
    instruments: Vec<InstrumentNameExchange>,
    _phantom: PhantomData<Server>,
}

impl<Server> BybitAccountStreamTransformer<Server> {
    pub fn new(assets: Vec<AssetNameExchange>, instruments: Vec<InstrumentNameExchange>) -> Self {
        Self {
            assets,
            instruments,
            _phantom: PhantomData,
        }
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

        // TODO: Should we filter events based on the instruments and assets?
        // events.into_iter().filter(|event| {
        //     if let Ok(event) = event {
        //         match &event.kind {
        //             AccountEventKind::Snapshot(account_snapshot) => todo!(),
        //             AccountEventKind::BalanceSnapshot(snapshot) => todo!(),
        //             AccountEventKind::OrderSnapshot(snapshot) => todo!(),
        //             AccountEventKind::OrderCancelled(order_event) => todo!(),
        //             AccountEventKind::Trade(trade) => todo!(),
        //         }
        //     }
        //     false
        // });

        events
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

pub async fn authenticate_connection(
    websocket: &mut WebSocket,
    credentials: &ApiCredentials,
) -> Result<(), UnindexedClientError> {
    // Send auth message
    websocket.send(auth_message(&credentials)).await?;

    let message = websocket.next().await.ok_or_else(|| {
        UnindexedClientError::AccountStream("Couldn't authenticate socket connection".to_string())
    })??;

    // TODO: Validate auth response
    dbg!(message);

    Ok(())
}

fn auth_message(credentials: &ApiCredentials) -> WsMessage {
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

pub async fn subscribe_topic(websocket: &mut WebSocket) -> Result<(), UnindexedClientError> {
    websocket.send(subscription_message()).await?;

    let message = websocket.next().await.ok_or_else(|| {
        UnindexedClientError::AccountStream("Couldn't subscribe to the topic".to_string())
    })??;

    // TODO Validate subscription response
    dbg!(message);

    Ok(())
}

fn subscription_message() -> WsMessage {
    WsMessage::text(
        serde_json::json!({
            "op": "subscribe",
            "args": ["order", "execution"] // TODO: Add account balance changes
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

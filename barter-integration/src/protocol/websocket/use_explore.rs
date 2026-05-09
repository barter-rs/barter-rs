use crate::socket::on_connect_err::ConnectErrorKind;
use crate::{
    Message,
    protocol::websocket::{AdminWs, WsError, explore::init_reconnecting_websocket},
    serde::de::Deserialiser,
    socket::{
        on_connect_err::{ConnectError, ConnectErrorAction},
        on_stream_err::StreamErrorAction,
    },
};
use bytes::Bytes;
use futures::{Sink, Stream};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;

fn use_init_reconnecting_websocket<De, AppMessage>(
    url: String,
    timeout_connect: std::time::Duration,
) -> Result<
    impl Stream<Item = impl Sink<AppMessage> + Stream<Item = Message<AdminWs, AppMessage>>>,
    WsError,
>
where
    De: Deserialiser<Bytes, AppMessage> + 'static,
    De::Error: Debug,
    AppMessage: Serialize + for<'de> Deserialize<'de> + Debug,
{
    let url_clone = url.clone();


    let on_connect_err = move |err_connect: &ConnectError<WsError>| match &err_connect.kind {
        ConnectErrorKind::Connect(error) => {
            warn!(
                %url,
                %error,
                action = "reconnecting after backoff",
                "failed to initialise WebSocket due to connect error"
            );
            ConnectErrorAction::Reconnect
        }
        ConnectErrorKind::Timeout => {
            warn!(
                %url,
                timeout = ?timeout_connect,
                action = "reconnecting after backoff",
                "failed to initialise WebSocket due to connect timeout"
            );
            ConnectErrorAction::Reconnect
        }
    };

    let on_stream_err = |error: &De::Error| {
        warn!(?error, "WebSocket payload deserialise error, dropping message");
        StreamErrorAction::Continue
    };

    init_reconnecting_websocket::<De, AppMessage, _, _>(
        url_clone,
        timeout_connect,
        on_connect_err,
        on_stream_err,
    )
}
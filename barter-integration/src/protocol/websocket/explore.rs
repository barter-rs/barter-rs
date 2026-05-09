// Interface:
// Protocol Layer management
// - Sink, impl Stream<Item = Bytes> or Item = T

use crate::socket::on_stream_err::{StreamErrorAction, StreamErrorHandler};
use crate::{
    protocol::websocket::{connect, AdminWs, WsError, WsMessage, WsParser},
    serde::{
        de::Deserialiser,
        se::SeJsonString,
    },
    socket::{
        backoff::DefaultBackoff, init_reconnecting_socket, on_connect_err::ConnectErrorHandler,
        ReconnectingSocket,
    },
    Message,
};
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::warn;
use crate::serde::se::Serialiser;
use crate::socket::on_connect_err::{ConnectError, ConnectErrorAction, ConnectErrorKind};

pub trait AdminWsStrategy {}
pub trait AdminAppStrategy {}

pub struct ConnectErrorHandlerWs {
    url: String,
    timeout_connect: std::time::Duration
}

fn default_on_connect_error_kind_ws(
    url: &str,
    error: &ConnectError<WsError>,
    timeout_connect: std::time::Duration,
) -> ConnectErrorAction {
    match error.kind {
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
    }
}

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


    let on_connect_err = |err_connect| default_on_connect_error_kind_ws(
        &url,
        err_connect,
        timeout_connect
    );

    // // Todo: probably let Result<(), ()> through for completeness
    // let on_stream_err = |error: &De::Error| {
    //     warn!(?error, "payload deserialise error, dropping message");
    //     StreamErrorAction::Continue
    // };

    init_reconnecting_websocket::<De, AppMessage, _, _>(
        url_clone,
        timeout_connect,
        on_connect_err,
        // on_stream_err,
    )
}

pub fn init_reconnecting_websocket<De, AppMessage, FnOnConnectErr, FnOnStreamErr>(
    url: String,
    timeout_connect: std::time::Duration,
    on_connect_err: FnOnConnectErr,
) -> Result<
    impl Stream<Item = impl Sink<AppMessage> + Stream<Item = Message<AdminWs, AppMessage>>>,
    WsError,
>
where
    De: Deserialiser<Bytes, AppMessage>,
    AppMessage: Serialize + for<'de> Deserialize<'de> + Debug,
    FnOnConnectErr: ConnectErrorHandler<WsError>,
    FnOnStreamErr: StreamErrorHandler<De::Error> + Clone + 'static,

{
    let connect = move || {
        let url = url.clone();
        async move {
            // Todo: need to find say way to apply a 'next event timeout' before flattening
            //       or don't flatten and let API caller handle
            init_websocket_serde::<De, AppMessage>(&url).await
        }
    };

    let stream = init_reconnecting_socket(connect, timeout_connect, DefaultBackoff)
        .on_connect_err(on_connect_err);

    Ok(stream)
}

// pub async fn init_socket<Se, De, SocketMessage>(
//
// )

pub async fn init_websocket_serde<De, AppMessage>(
    url: &str,
) -> Result<
    impl Sink<AppMessage> + Stream<Item = Message<AdminWs, Result<AppMessage, De::Error>>> + use<De, AppMessage>,
    WsError,
>
where
    De: Deserialiser<Bytes, AppMessage>,
    AppMessage: Serialize + for<'de> Deserialize<'de> + Debug,
{
    let socket = connect(url).await?.map(WsParser::parse);

    Ok(with_serde::<De, AppMessage>(socket))
}

// actual Stream & Sink impl?
// pub struct SerdeSocket<Socket> {
//     inner: Socket,
//     se: Se,
//     de: De,
// }

pub fn with_serde<Se, De, AppMessage, Wire>(
    socket: impl Sink<Wire> + Stream<Item = Wire>
) -> impl Sink<AppMessage> + Stream<Item = Result<Message<AdminWs, AppMessage>, De::Error>>
where
    Se: Serialiser<AppMessage, Wire>,
    De: Deserialiser<Wire, AppMessage>,
{
    socket
        .with(|message: AppMessage| async move {
            SeJsonString::se_string(&message)
                .map(WsMessage::text)
                .map_err(WsSinkError::Serialise)
        })
        .map(|message| match message {
            Message::Admin(admin) => Message::Admin(admin),
            Message::Payload(payload) => Message::Payload(De::deserialise(payload)),
        })

}


pub fn with_serde<De, AppMessage>(
    socket: impl Sink<WsMessage, Error = WsError> + Stream<Item = Message<AdminWs, Bytes>>,
) -> impl Sink<AppMessage> + Stream<Item = Result<Message<AdminWs, AppMessage>, De::Error>>
where
    De: Deserialiser<Bytes, AppMessage>,
    AppMessage: Serialize + for<'de> Deserialize<'de> + Debug,
{
    socket
        .with(|message: AppMessage| async move {
            SeJsonString::se_string(&message)
                .map(WsMessage::text)
                .map_err(WsSinkError::Serialise)
        })
        .map(|message| match message {
            Message::Admin(admin) => Message::Admin(admin),
            Message::Payload(payload) => Message::Payload(De::deserialise(payload)),
        })
}

pub enum WsSinkError<SeErr> {
    Sink(WsError),
    Serialise(SeErr),
}

impl<SeError> From<WsError> for WsSinkError<SeError> {
    fn from(value: WsError) -> Self {
        Self::Sink(value)
    }
}

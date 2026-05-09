// Interface:
// Protocol Layer management
// - Sink, impl Stream<Item = Bytes> or Item = T

use crate::socket::on_stream_err::StreamErrorHandler;
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

pub trait AdminWsStrategy {}
pub trait AdminAppStrategy {}

pub fn init_reconnecting_websocket<De, AppMessage, FnOnConnectErr, FnOnStreamErr>(
    url: String,
    timeout_connect: std::time::Duration,
    on_connect_err: FnOnConnectErr,
    on_stream_err: FnOnStreamErr,
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
        .on_connect_err(on_connect_err)
        .on_stream_err_filter(on_stream_err);

    Ok(stream)
}

pub async fn init_websocket_serde<De, AppMessage>(
    url: &str,
) -> Result<
    impl Sink<AppMessage> + Stream<Item = Result<Message<AdminWs, AppMessage>, De::Error>> + use<De, AppMessage>,
    WsError,
>
where
    De: Deserialiser<Bytes, AppMessage>,
    AppMessage: Serialize + for<'de> Deserialize<'de> + Debug,
{
    let socket = connect(url).await?.map(WsParser::parse);

    Ok(with_serde::<De, AppMessage>(socket))
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
            Message::Admin(admin) => Ok(Message::Admin(admin)),
            Message::Payload(payload) => De::deserialise(payload).map(Message::Payload),
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

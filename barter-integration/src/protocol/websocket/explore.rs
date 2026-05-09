// Interface:
// Protocol Layer management
// - Sink, impl Stream<Item = Bytes> or Item = T

use crate::{
    Message,
    protocol::websocket::{AdminWs, WsError, WsMessage, WsParser, WsSink, connect},
    serde::{
        de::{DeJson, Deserialiser, error::DeBinaryError},
        se::SeJsonString,
    },
    socket::{
        ReconnectingSocket, backoff::DefaultBackoff, init_reconnecting_socket,
        on_connect_err::ConnectErrorHandler,
    },
};
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};

pub trait AdminWsStrategy {}
pub trait AdminAppStrategy {}

fn init_reconnecting_websocket<FnOnConnectErr, AppMessage>(
    url: String,
    timeout_connect: std::time::Duration,
    on_connect_err: FnOnConnectErr,
) -> Result<
    impl Stream<Item = impl Sink<WsMessage> + Stream<Item = Message<AdminWs, bytes::Bytes>>>,
    WsError,
>
where
    FnOnConnectErr: ConnectErrorHandler<WsError>,
{
    let connect = move || {
        let url = url.clone();
        async move {
            // Todo: need to find say way to apply a 'next event timeout' before flattening
            //       or don't flatten and let API caller handle
            init_websocket_serde(&url).await
        }
    };

    let stream = init_reconnecting_socket(connect, timeout_connect, DefaultBackoff)
        .on_connect_err(on_connect_err);

    Ok(stream)
}

pub async fn init_websocket_serde<De, AppMessage>(
    url: &str,
) -> Result<impl Sink<AppMessage> + Stream<Item = Message<AdminWs, AppMessage>>, WsError>
where
    De: Deserialiser<Bytes, AppMessage>,
    AppMessage: Serialize + for<'de> Deserialize<'de>,
{
    let socket = connect(url).await?.map(WsParser::parse);

    Ok(with_serde::<De, AppMessage>(socket))
}

pub fn with_serde<De, AppMessage>(
    socket: impl Sink<WsMessage> + Stream<Item = Message<AdminWs, Bytes>>,
) -> impl Sink<AppMessage> + Stream<Item = Result<Message<AdminWs, AppMessage>, DeBinaryError>>
where
    De: Deserialiser<Bytes, AppMessage>,
    AppMessage: Serialize + for<'de> Deserialize<'de>,
{
    use futures::{SinkExt, StreamExt};

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
    Sink(WsSink),
    Serialise(SeErr),
}

impl<SeError> From<WsSink> for WsSinkError<SeError> {
    fn from(value: WsSink) -> Self {
        Self::Sink(value)
    }
}

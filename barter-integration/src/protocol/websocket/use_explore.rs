use std::time::Duration;
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use crate::Message;
use crate::protocol::websocket::{AdminWs, WsError, WsMessage};
use crate::protocol::websocket::use_explore_protocol_layer::PongTracker;
use crate::serde::de::Deserialiser;
use crate::stream::util::merge::merge;

pub struct WsSinkManager<Sink> {
    sink: Sink,
}

// Todo: Create some management component which leverages the output of the init functions.
//
//  ie/ Handles: impl Sink<AppMessage> + Stream<Item = Message<AdminWs, AppMessage>>

fn manage_ws<AppMessage>(
    to_send_stream: impl Stream<Item = AppMessage>,
    socket: impl Sink<AppMessage> + Stream<Item = Message<AdminWs, AppMessage>>
)

{

}


// High-level API for users upon init will return:
//  - impl Stream<Item = AppMessage>
//  - Tx<Item = Command<AppMessage>, where Command would be commands related to Socket

enum SocketCommand<ToSend> {
    Send(ToSend),
    Close,
    // Todo: are there any more?
}

pub struct HighLevelWsConcreteApi;

impl HighLevelWsConcreteApi {
    fn init<AppMessage>(

    ) -> (
        mpsc::Sender<WsMessage>,
        impl Stream<Item = AppMessage>
    ) {

        todo!()
    }
}

pub fn ping_stream(period: Duration) -> impl Stream<Item = WsMessage> {
    IntervalStream::new(interval(period)).map(|_| WsMessage::Ping(Bytes::new()))
}



// Todo: This is the crux
fn route_admin_ws_to_manager(
    stream: impl Stream<Item = WsMessage>,
    fn_route: impl FnMut(AdminWs),
) -> impl Stream<Item = Bytes>
{

}
// Which leads to:
pub async fn manage_websocket(
    stream: impl Stream<Item = AdminWs>,
    ping_interval: std::time::Duration,
    pong_timeout: std::time::Duration,
    fn_route: impl FnMut(WsMessage) // Where FnRoute goes to manage_sink
) // This is a terminal operation for the Stream
{
    let pings = IntervalStream::new(tokio::time::interval(ping_interval))
        .map(|_| Message::Payload(WsMessage::Ping(Bytes::new())));

    // Todo: need to add 'heartbeat' for Pong response check!
    //  Will need to merge this in... or use select since this isn't the 'consumer stream' side.
    let pong_check = IntervalStream::new(tokio::time::interval(pong_timeout / 2));

    let mut stream = merge(
        stream.map(Message::Admin),
        pings,
    );

    let mut pong_tracker = PongTracker::new(pong_timeout);

    while let Some(work) = stream.next().await {
        let admin = match work {
            Message::Admin(admin) => admin,
            Message::Payload(ping_to_send) => {
                pong_tracker.record_ping_sent();
                fn_route(ping_to_send);
                continue;
            }
        };

        match admin {
            AdminWs::Ping(_ping) => {
                // tokio_tungstenite auto-pongs
            }
            AdminWs::Pong(_pong) => {
                pong_tracker.record_pong_received();
            }
            AdminWs::Close(close_frame) => {
                // Todo: How to handle gracefully
            }
            AdminWs::WsError(error) => {
                // Todo: Handle error gracefull
            }
        }

    }


}
// Todo: Since manage_sink is just a pass-through, we need a Stream processor that that listens to
//  Stream<Item = WsMessage || Message<AdminWs, Bytes> (maybe WsMessage is actually better?),
//  and:
//  1. Payloads need to pass through this layer as efficiently as possible.
//  2. Non-payloads are
// So, non-payloads could be routed from main stream processing, to the 'websocket admin processor'
// which takes Stream<Item = AdminWs> and has the ability to hold state (eg/ last ping), and forward
// WsMessages to the 'sink'.
pub async fn manage_sink<S, SinkItem>(
    sink: S,
    to_send: impl Stream<Item = SinkItem>, // Todo: merge ping stream + others
) -> Result<(), S::Error>
where
    S: Sink<SinkItem>,
{
    to_send
        .map(Ok)
        .forward(sink)
        .await
}


struct AdminWsManager {
    pong_timeout: PongTracker,
}

impl AdminWsManager {

}




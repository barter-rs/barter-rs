use barter_instrument::Keyed;
use barter_integration::{
    Message,
    protocol::websocket::{AdminWs, WsParser, connect},
    stream::manager::StreamManager,
};
use bytes::Bytes;
use futures::{Stream, StreamExt, future::try_join_all};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum StreamKey {
    Binance,
    Bitstamp,
}

#[derive(Clone)]
struct StreamArgs {
    url: String,
}

impl StreamArgs {
    fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_args = [
        (
            StreamKey::Binance,
            StreamArgs::new("wss://stream.binance.com:9443/ws"),
        ),
        (
            StreamKey::Bitstamp,
            StreamArgs::new("wss://ws.bitstamp.net"),
        ),
    ];

    let runtime = tokio::runtime::Handle::current();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let consumer_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            println!("{event:?}");
        }
    });

    let mut manager = StreamManager::init(stream_args, init_stream)
        .await?
        .route_streams(runtime, move |_stream_key| {
            let tx = tx.clone();
            move |stream_key, item| tx.send(Keyed::new(stream_key.clone(), item))
        });

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    manager
        .shutdown(&StreamKey::Bitstamp)
        .expect("Bitstamp is in not map")
        .component
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let shutdown_futures = manager
        .shutdown_all()
        .map(|(_stream_key, value)| value.component);

    let _ = try_join_all(shutdown_futures).await?;
    consumer_task.await?;

    Ok(())
}

async fn init_stream(
    key: &StreamKey,
    args: &StreamArgs,
) -> Result<Box<dyn Stream<Item = Message<AdminWs, Bytes>> + Send + Unpin>, String> {
    connect(args.url.as_str())
        .await
        .map(|socket| {
            let stream = Box::pin(socket.map(WsParser::parse));
            Box::new(stream) as Box<dyn Stream<Item = Message<AdminWs, Bytes>> + Send + Unpin>
        })
        .map_err(|error| format!("{key:?}: {error}"))
}

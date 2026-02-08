use barter_instrument::Keyed;
use barter_integration::{
    Message,
    protocol::websocket::{AdminWs, WsParser, connect},
    stream::manager::StreamManager,
    task::{TokioTaskHandle, TokioTaskManager},
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
    let entries = [
        (
            StreamKey::Binance,
            StreamArgs::new("wss://stream.binance.com:9443/ws"),
        ),
        (
            StreamKey::Bitstamp,
            StreamArgs::new("wss://ws.bitstamp.net"),
        ),
    ];

    let task_manager =
        TokioTaskManager::init(entries, |key: &StreamKey, args: &StreamArgs| async {
            let stream = init_stream(key, args).await?;

            Ok(TokioTaskHandle {
                handle: (),
                shutdown_tx: (),
            })
        })
        .await?;

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
) -> Result<impl Stream<Item = Message<AdminWs, Bytes>>, String> {
    connect(args.url.as_str())
        .await
        .map(|socket| socket.map(WsParser::parse))
        .map_err(|error| format!("{key:?}: {error}"))
}

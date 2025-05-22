use tokio_stream::{StreamExt as TokioStreamExt};
use futures::StreamExt;
use jackbot_integration::protocol::websocket::{WsMessage, WsError};
use std::time::Duration;
use std::io;

#[tokio::test]
async fn test_heartbeat_timeout_results_in_error() {
    tokio::time::pause();

    let stream = tokio_stream::pending::<Result<WsMessage, WsError>>()
        .timeout(Duration::from_secs(1))
        .map(|res| match res {
            Ok(msg) => msg,
            Err(_) => Err(WsError::Io(io::Error::new(io::ErrorKind::TimedOut, "heartbeat timeout"))),
        });

    tokio::pin!(stream);

    tokio::time::advance(Duration::from_secs(2)).await;

    match stream.next().await {
        Some(Err(WsError::Io(err))) => {
            assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

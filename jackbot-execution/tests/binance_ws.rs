use jackbot_execution::{
    client::{binance::{BinanceWsClient, BinanceWsConfig}, ExecutionClient},
    AccountEventKind,
};
use tokio::{net::TcpListener};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use url::Url;

async fn run_server(addr: &str, first: String, second: String) {
    let listener = TcpListener::bind(addr).await.unwrap();
    for payload in [first, second] {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        // recv auth
        ws.next().await.unwrap().unwrap();
        ws.send(Message::Text(payload)).await.unwrap();
        ws.close(None).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reconnect_and_normalise() {
    let addr = "127.0.0.1:18080";
    let first = r#"{\"e\":\"balance\",\"E\":1,\"asset\":\"BTC\",\"free\":\"0.5\",\"total\":\"1.0\"}"#.to_string();
    let second = r#"{\"e\":\"order\",\"E\":2,\"s\":\"BTCUSDT\",\"S\":\"BUY\",\"p\":\"100\",\"q\":\"0.1\",\"i\":1,\"X\":\"NEW\"}"#.to_string();
    tokio::spawn(run_server(addr, first.clone(), second.clone()));

    let client = BinanceWsClient::new(BinanceWsConfig {
        url: Url::parse(&format!("ws://{}", addr)).unwrap(),
        auth_payload: "{}".to_string(),
    });
    let mut stream = client.account_stream(&[], &[]).await.unwrap();

    let ev1 = stream.next().await.unwrap();
    match ev1.kind {
        AccountEventKind::BalanceSnapshot(_) => {}
        _ => panic!("expected balance"),
    }
    let ev2 = stream.next().await.unwrap();
    match ev2.kind {
        AccountEventKind::OrderSnapshot(_) => {}
        _ => panic!("expected order"),
    }
}

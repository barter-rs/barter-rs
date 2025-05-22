use jackbot_execution::{
    client::{coinbase::{CoinbaseWsClient, CoinbaseWsConfig}, ExecutionClient},
    AccountEventKind,
};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use url::Url;

async fn run_server(addr: &str, first: String, second: String, third: String) {
    let listener = TcpListener::bind(addr).await.unwrap();
    for payload in [first, second, third] {
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
    let addr = "127.0.0.1:18180";
    let balance = r#"{\"type\":\"balance\",\"time\":1,\"asset\":\"BTC\",\"free\":\"0.5\",\"total\":\"1.0\"}"#.to_string();
    let order = r#"{\"type\":\"order\",\"time\":2,\"product_id\":\"BTC-USD\",\"side\":\"buy\",\"price\":\"100\",\"size\":\"0.1\",\"order_id\":\"1\",\"status\":\"NEW\"}"#.to_string();
    let fill = r#"{\"type\":\"fill\",\"time\":3,\"trade_id\":1,\"product_id\":\"BTC-USD\",\"side\":\"buy\",\"price\":\"100\",\"size\":\"0.1\"}"#.to_string();
    tokio::spawn(run_server(addr, balance.clone(), order.clone(), fill.clone()));

    let client = CoinbaseWsClient::new(CoinbaseWsConfig {
        url: Url::parse(&format!("ws://{}", addr)).unwrap(),
        auth_payload: "{}".to_string(),
    });
    let mut stream = client.account_stream(&[], &[]).await.unwrap();

    match stream.next().await.unwrap().kind {
        AccountEventKind::BalanceSnapshot(_) => {}
        _ => panic!("expected balance"),
    }
    match stream.next().await.unwrap().kind {
        AccountEventKind::OrderSnapshot(_) => {}
        _ => panic!("expected order"),
    }
    match stream.next().await.unwrap().kind {
        AccountEventKind::Trade(_) => {}
        _ => panic!("expected trade"),
    }
}

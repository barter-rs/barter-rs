use jackbot_data::exchange::{
    binance::{channel::BinanceChannel, market::BinanceMarket, spot::BinanceSpot},
    subscription::ExchangeSub,
};
use tokio_tungstenite::tungstenite::Message;

#[test]
fn test_binance_spot_trade_requests() {
    let subs = vec![
        ExchangeSub::from((BinanceChannel::TRADES, BinanceMarket("BTCUSDT".into()))),
        ExchangeSub::from((BinanceChannel::TRADES, BinanceMarket("ETHUSDT".into()))),
    ];

    let msgs = BinanceSpot::requests(subs);
    assert_eq!(msgs.len(), 1);
    match &msgs[0] {
        Message::Text(text) => {
            let v: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(v["method"], "SUBSCRIBE");
            assert_eq!(v["id"], 1);
            assert_eq!(v["params"], serde_json::json!(["btcusdt@trade", "ethusdt@trade"]));
        }
        _ => panic!("expected text message"),
    }
}

#[test]
fn test_binance_spot_trade_requests_empty() {
    let msgs = BinanceSpot::requests(Vec::<ExchangeSub<_, _>>::new());
    assert_eq!(msgs.len(), 1);
    match &msgs[0] {
        Message::Text(text) => {
            let v: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(v["params"].as_array().unwrap().len(), 0);
        }
        _ => panic!("expected text message"),
    }
}

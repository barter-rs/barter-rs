use jackbot_data::{
    books::{manager::OrderBookL2Manager, map::OrderBookMap},
    redis_store::InMemoryStore,
    subscription::book::OrderBookEvent,
    streams::consumer::MarketStreamEvent,
    Identifier,
};
use jackbot_instrument::{
    exchange::ExchangeId,
    instrument::market_data::{MarketDataInstrument, kind::MarketDataInstrumentKind},
};
use rust_decimal_macros::dec;
use chrono::Utc;
use futures::Stream;
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_store_snapshot_and_delta() {
    let instrument = MarketDataInstrument::new("btc", "usdt", MarketDataInstrumentKind::Spot);
    let events = vec![
        MarketStreamEvent::Item(jackbot_data::event::MarketEvent {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange: ExchangeId::BinanceSpot,
            instrument: instrument.clone(),
            kind: OrderBookEvent::Snapshot(jackbot_data::books::OrderBook::default()),
        }),
        MarketStreamEvent::Reconnecting(ExchangeId::BinanceSpot),
        MarketStreamEvent::Item(jackbot_data::event::MarketEvent {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange: ExchangeId::BinanceSpot,
            instrument: instrument.clone(),
            kind: OrderBookEvent::Update(jackbot_data::books::OrderBook::default()),
        }),
    ];

    let stream = futures::stream::iter(events);

    let mut map = jackbot_data::books::map::OrderBookMapMulti::new(Default::default());
    map.insert(instrument.clone(), std::sync::Arc::new(parking_lot::RwLock::new(jackbot_data::books::OrderBook::default())));

    let store = InMemoryStore::new();

    let manager = OrderBookL2Manager { stream, books: map, store: store.clone() };

    manager.run().await;

    assert!(store.get_snapshot_json(ExchangeId::BinanceSpot, &instrument.to_string()).is_some());
    assert_eq!(store.delta_len(ExchangeId::BinanceSpot, &instrument.to_string()), 1);
}

#[tokio::test]
async fn test_reconnect_overwrites_snapshot() {
    let instrument = MarketDataInstrument::new("btc", "usdt", MarketDataInstrumentKind::Spot);
    let events = vec![
        MarketStreamEvent::Item(jackbot_data::event::MarketEvent {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange: ExchangeId::BinanceSpot,
            instrument: instrument.clone(),
            kind: OrderBookEvent::Snapshot(jackbot_data::books::OrderBook::default()),
        }),
        MarketStreamEvent::Reconnecting(ExchangeId::BinanceSpot),
        MarketStreamEvent::Item(jackbot_data::event::MarketEvent {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange: ExchangeId::BinanceSpot,
            instrument: instrument.clone(),
            kind: OrderBookEvent::Snapshot(jackbot_data::books::OrderBook::default()),
        }),
        MarketStreamEvent::Item(jackbot_data::event::MarketEvent {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange: ExchangeId::BinanceSpot,
            instrument: instrument.clone(),
            kind: OrderBookEvent::Update(jackbot_data::books::OrderBook::default()),
        }),
    ];

    let stream = futures::stream::iter(events);

    let mut map = jackbot_data::books::map::OrderBookMapMulti::new(Default::default());
    map.insert(instrument.clone(), std::sync::Arc::new(parking_lot::RwLock::new(jackbot_data::books::OrderBook::default())));

    let store = InMemoryStore::new();

    let manager = OrderBookL2Manager { stream, books: map, store: store.clone() };

    manager.run().await;

    assert!(store.get_snapshot_json(ExchangeId::BinanceSpot, &instrument.to_string()).is_some());
    assert_eq!(store.delta_len(ExchangeId::BinanceSpot, &instrument.to_string()), 1);
}

#[test]
fn test_mexc_store_methods() {
    use jackbot_data::exchange::mexc::spot::l2::MexcOrderBookL2;
    use jackbot_data::exchange::mexc::futures::l2::MexcFuturesOrderBookL2;

    let store = InMemoryStore::new();

    let spot_book = MexcOrderBookL2 {
        subscription_id: "BTC_USDT".into(),
        time: Utc::now(),
        bids: vec![(dec!(30000.0), dec!(1.0))],
        asks: vec![(dec!(30010.0), dec!(2.0))],
    };
    spot_book.store_snapshot(&store);
    assert!(store.get_snapshot_json(ExchangeId::Mexc, "BTC_USDT").is_some());

    let delta_book = MexcOrderBookL2 { time: Utc::now(), ..spot_book };
    delta_book.store_delta(&store);
    assert_eq!(store.delta_len(ExchangeId::Mexc, "BTC_USDT"), 1);

    let fut_book = MexcFuturesOrderBookL2 {
        subscription_id: "BTC_USDT".into(),
        time: Utc::now(),
        bids: vec![(dec!(30000.0), dec!(1.0))],
        asks: vec![(dec!(30010.0), dec!(2.0))],
    };
    fut_book.store_snapshot(&store);
    fut_book.store_delta(&store);

    assert!(store.get_snapshot_json(ExchangeId::Mexc, "BTC_USDT").is_some());
    assert_eq!(store.delta_len(ExchangeId::Mexc, "BTC_USDT"), 2);
}

#[test]


fn test_query_methods() {
    use jackbot_data::books::{OrderBook, Level};
    use jackbot_data::subscription::book::OrderBookEvent;

    let store = InMemoryStore::new();
    let snapshot = OrderBook::new(0, None, [Level::new(dec!(1), dec!(1))].into_iter(), []);
    store.store_snapshot(ExchangeId::BinanceSpot, "BTC_USDT", &snapshot);
    let delta = OrderBookEvent::Update(OrderBook::new(0, None, [], []));
    store.store_delta(ExchangeId::BinanceSpot, "BTC_USDT", &delta);
    store.store_trade(ExchangeId::BinanceSpot, "BTC_USDT", &jackbot_data::subscription::trade::PublicTrade {
        id: "1".into(), price: 1.0, amount: 1.0, side: jackbot_instrument::Side::Buy });

    assert!(store.get_snapshot(ExchangeId::BinanceSpot, "BTC_USDT").is_some());
    assert_eq!(store.get_deltas(ExchangeId::BinanceSpot, "BTC_USDT", 1).len(), 1);
    assert_eq!(store.get_trades(ExchangeId::BinanceSpot, "BTC_USDT", 1).len(), 1);
}


fn test_cryptocom_store_methods() {
    use jackbot_data::exchange::cryptocom::spot::l2::CryptocomOrderBookL2;
    use jackbot_data::exchange::cryptocom::futures::l2::CryptocomFuturesOrderBookL2;

    let store = InMemoryStore::new();

    let spot_book = CryptocomOrderBookL2 {
        subscription_id: "BTC_USDT".into(),
        time: Utc::now(),
        bids: vec![(dec!(30000.0), dec!(1.0))],
        asks: vec![(dec!(30010.0), dec!(2.0))],
    };
    spot_book.store_snapshot(&store);
    assert!(store.get_snapshot(ExchangeId::Cryptocom, "BTC_USDT").is_some());

    let delta_book = CryptocomOrderBookL2 { time: Utc::now(), ..spot_book };
    delta_book.store_delta(&store);
    assert_eq!(store.delta_len(ExchangeId::Cryptocom, "BTC_USDT"), 1);

    let fut_book = CryptocomFuturesOrderBookL2 {
        subscription_id: "BTC_USDT".into(),
        time: Utc::now(),
        bids: vec![(dec!(30000.0), dec!(1.0))],
        asks: vec![(dec!(30010.0), dec!(2.0))],
    };
    fut_book.store_snapshot(&store);
    fut_book.store_delta(&store);

    assert!(store.get_snapshot(ExchangeId::Cryptocom, "BTC_USDT").is_some());
    assert_eq!(store.delta_len(ExchangeId::Cryptocom, "BTC_USDT"), 2);
}

#[test]
fn test_gateio_store_methods() {
    use jackbot_data::exchange::gateio::spot::l2::GateioOrderBookL2;
    use jackbot_data::exchange::gateio::futures::l2::GateioFuturesOrderBookL2;

    let store = InMemoryStore::new();

    let spot_book = GateioOrderBookL2 {
        subscription_id: "BTC_USDT".into(),
        time: Utc::now(),
        bids: vec![(dec!(30000.0), dec!(1.0))],
        asks: vec![(dec!(30010.0), dec!(2.0))],
    };
    spot_book.store_snapshot(&store);
    assert!(store.get_snapshot(ExchangeId::Gateio, "BTC_USDT").is_some());

    let delta_book = GateioOrderBookL2 { time: Utc::now(), ..spot_book };
    delta_book.store_delta(&store);
    assert_eq!(store.delta_len(ExchangeId::Gateio, "BTC_USDT"), 1);

    let fut_book = GateioFuturesOrderBookL2 {
        subscription_id: "BTC_USDT".into(),
        time: Utc::now(),
        bids: vec![(dec!(30000.0), dec!(1.0))],
        asks: vec![(dec!(30010.0), dec!(2.0))],
    };
    fut_book.store_snapshot(&store);
    fut_book.store_delta(&store);

    assert!(store.get_snapshot(ExchangeId::Gateio, "BTC_USDT").is_some());
    assert_eq!(store.delta_len(ExchangeId::Gateio, "BTC_USDT"), 2);
}

#[test]
fn test_list_trimming() {
    use jackbot_data::books::OrderBook;
    use jackbot_data::subscription::book::OrderBookEvent;

    let store = InMemoryStore::new();
    for _ in 0..(jackbot_data::redis_store::MAX_LIST_LEN + 10) {
        store.store_delta(ExchangeId::BinanceSpot, "BTC_USDT", &OrderBookEvent::Update(OrderBook::default()));
        store.store_trade(ExchangeId::BinanceSpot, "BTC_USDT", &jackbot_data::subscription::trade::PublicTrade {
            id: "1".into(),
            price: 1.0,
            amount: 1.0,
            side: jackbot_instrument::Side::Buy,
        });
    }

    assert_eq!(store.delta_len(ExchangeId::BinanceSpot, "BTC_USDT"), jackbot_data::redis_store::MAX_LIST_LEN);
    assert_eq!(store.get_trades(ExchangeId::BinanceSpot, "BTC_USDT", usize::MAX).len(), jackbot_data::redis_store::MAX_LIST_LEN);
}


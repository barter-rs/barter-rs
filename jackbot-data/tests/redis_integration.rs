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

    assert!(store.get_snapshot(ExchangeId::BinanceSpot, &instrument.to_string()).is_some());
    assert_eq!(store.delta_len(ExchangeId::BinanceSpot, &instrument.to_string()), 1);
}

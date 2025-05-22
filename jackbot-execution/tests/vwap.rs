use jackbot_execution::{
    vwap::{vwap_slices, VwapScheduler},
    client::{binance::futures::{BinanceFuturesUsd, BinanceFuturesUsdConfig}, mock::{MockExecution, MockExecutionClientConfig, MockExecutionConfig}},
    exchange::mock::MockExchange,
    order::{
        id::{ClientOrderId, StrategyId},
        request::{OrderRequestOpen, RequestOpen},
        OrderKey, OrderKind, TimeInForce,
    },
};
use jackbot_data::books::{OrderBook, Level, aggregator::{OrderBookAggregator, ExchangeBook}};
use jackbot_instrument::{exchange::ExchangeId, instrument::{Instrument, name::InstrumentNameExchange}, Underlying};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rust_decimal_macros::dec;
use rust_decimal::Decimal;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::{mpsc, broadcast};
use chrono::Utc;
use tokio::time::Duration;

#[test]
fn test_vwap_slices_sum() {
    let mut rng = StdRng::seed_from_u64(7);
    let vols = vec![dec!(2), dec!(1), dec!(7)];
    let parts = vwap_slices(dec!(10), &vols, 0.2, &mut rng);
    assert_eq!(parts.len(), 3);
    let total: rust_decimal::Decimal = parts.iter().copied().sum();
    assert_eq!(total, dec!(10));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_vwap_scheduler_mock_exchange() {
    let book = OrderBook::new(0, None, vec![Level::new(dec!(100), dec!(1))], vec![Level::new(dec!(101), dec!(1))]);
    let book = Arc::new(RwLock::new(book));
    let aggregator = OrderBookAggregator::new([ExchangeBook { exchange: ExchangeId::BinanceSpot, book: book.clone(), weight: Decimal::ONE }]);

    let instrument = Instrument::spot(
        ExchangeId::BinanceSpot,
        "btc_usdt",
        "BTC-USDT",
        Underlying::new("btc", "usdt"),
        None,
    );
    let mut instruments = fnv::FnvHashMap::default();
    instruments.insert(instrument.name_exchange.clone(), instrument);

    let snapshot = jackbot_execution::UnindexedAccountSnapshot { exchange: ExchangeId::BinanceSpot, balances: Vec::new(), instruments: Vec::new() };
    let config_exchange = MockExecutionConfig { mocked_exchange: ExchangeId::BinanceSpot, initial_state: snapshot, latency_ms: 0, fees_percent: dec!(0) };
    let (req_tx, req_rx) = mpsc::unbounded_channel();
    let (event_tx, _event_rx) = broadcast::channel(8);
    let exchange = MockExchange::new(config_exchange, req_rx, event_tx.clone(), instruments);
    tokio::spawn(exchange.run());

    let config_client = MockExecutionClientConfig::new(ExchangeId::BinanceSpot, || Utc::now(), req_tx, event_tx.subscribe());
    let client = MockExecution::new(config_client);

    let mut scheduler = VwapScheduler::new(client, aggregator, StdRng::seed_from_u64(1));
    let vols = [dec!(1), dec!(2)];
    let request = OrderRequestOpen {
        key: OrderKey {
            exchange: ExchangeId::BinanceSpot,
            instrument: InstrumentNameExchange::from("BTC-USDT"),
            strategy: StrategyId::new("vwap"),
            cid: ClientOrderId::new("cid"),
        },
        state: RequestOpen {
            side: jackbot_instrument::Side::Buy,
            price: dec!(100),
            quantity: dec!(3),
            kind: OrderKind::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
        },
    };

    let results = scheduler.execute(request, &vols, 0.1, Duration::from_millis(1)).await;
    assert_eq!(results.len(), 2);
}

#[test]
fn test_vwap_scheduler_real_client_compile() {
    let client = BinanceFuturesUsd::new(BinanceFuturesUsdConfig::default());
    let aggregator = OrderBookAggregator::default();
    let _scheduler = VwapScheduler::new(client, aggregator, StdRng::seed_from_u64(7));
}

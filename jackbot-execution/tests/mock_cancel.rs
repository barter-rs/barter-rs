use jackbot_execution::{
    exchange::mock::MockExchange,
    client::mock::MockExecutionConfig,
    error::UnindexedOrderError,
    order::{
        id::{ClientOrderId, StrategyId},
        request::{OrderRequestCancel, RequestCancel},
        OrderKey,
    },
    UnindexedAccountSnapshot,
};
use jackbot_instrument::{
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use fnv::FnvHashMap;
use rust_decimal_macros::dec;
use tokio::sync::{broadcast, mpsc};

#[test]
fn cancel_order_returns_rejected_error() {
    let snapshot = UnindexedAccountSnapshot {
        exchange: ExchangeId::BinanceSpot,
        balances: Vec::new(),
        instruments: Vec::new(),
    };
    let config = MockExecutionConfig {
        mocked_exchange: ExchangeId::BinanceSpot,
        initial_state: snapshot,
        latency_ms: 0,
        fees_percent: dec!(0),
    };
    let (_tx, rx) = mpsc::unbounded_channel();
    let (event_tx, _rx) = broadcast::channel(1);
    let mut exchange = MockExchange::new(config, rx, event_tx, FnvHashMap::default());

    let request = OrderRequestCancel {
        key: OrderKey {
            exchange: ExchangeId::BinanceSpot,
            instrument: InstrumentNameExchange::from("BTC-USDT"),
            strategy: StrategyId::unknown(),
            cid: ClientOrderId::new("1"),
        },
        state: RequestCancel { id: None },
    };

    let response = exchange.cancel_order(request);
    assert!(matches!(response.state, Err(UnindexedOrderError::Rejected(_))));
}

use jackbot::strategy::advanced_orders::{twap_slices, vwap_slices, AlwaysMakerStrategy, BestBidAsk};
use jackbot::strategy::algo::AlgoStrategy;
use jackbot::engine::state::{EngineState, builder::EngineStateBuilder};
use jackbot_instrument::{
    Side,
    instrument::{Instrument, spec::{InstrumentSpec, InstrumentSpecPrice, InstrumentSpecQuantity, InstrumentSpecNotional, OrderQuantityUnits},},
    Underlying,
    exchange::ExchangeId,
};
use jackbot_instrument::index::IndexedInstruments;
use jackbot_execution::order::{OrderKind, TimeInForce};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rand::SeedableRng;
use rand::rngs::StdRng;

#[test]
fn test_twap_slices_sum() {
    let mut rng = StdRng::seed_from_u64(42);
    let parts = twap_slices(dec!(10), 5, 0.2, &mut rng);
    assert_eq!(parts.len(), 5);
    let total: Decimal = parts.iter().copied().sum();
    assert_eq!(total, dec!(10));
}

#[test]
fn test_vwap_slices_sum() {
    let mut rng = StdRng::seed_from_u64(7);
    let vols = vec![dec!(2), dec!(1), dec!(7)];
    let parts = vwap_slices(dec!(10), &vols, 0.2, &mut rng);
    assert_eq!(parts.len(), 3);
    let total: Decimal = parts.iter().copied().sum();
    assert_eq!(total, dec!(10));
}

#[test]
fn test_always_maker_strategy_new_order() {
    let instruments = IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .build();

    let mut state: EngineState<(), crate::engine::state::instrument::data::DefaultInstrumentMarketData> =
        EngineStateBuilder::new(&instruments, (), crate::engine::state::instrument::data::DefaultInstrumentMarketData::default)
            .build();

    // seed orderbook
    let instrument = state.instruments.instrument_index_mut(&jackbot_instrument::instrument::InstrumentIndex(0));
    instrument.data.l1.upsert_bids(vec![(dec!(100), dec!(1))]);
    instrument.data.l1.upsert_asks(vec![(dec!(101), dec!(1))]);

    let strat = AlwaysMakerStrategy { id: jackbot_execution::order::id::StrategyId::new("test"), side: Side::Buy, quantity: dec!(1) };
    let (_cancels, opens) = strat.generate_algo_orders(&state);
    let order = opens.into_iter().next().expect("order");
    assert_eq!(order.state.price, dec!(100));
    assert_eq!(order.state.kind, OrderKind::Limit);
    assert!(matches!(order.state.time_in_force, TimeInForce::GoodUntilCancelled { post_only: true }));
}

#[test]
fn test_always_maker_strategy_repost() {
    let instruments = IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .build();

    let mut state: EngineState<(), crate::engine::state::instrument::data::DefaultInstrumentMarketData> =
        EngineStateBuilder::new(&instruments, (), crate::engine::state::instrument::data::DefaultInstrumentMarketData::default)
            .build();

    // initial book
    {
        let instrument = state.instruments.instrument_index_mut(&jackbot_instrument::instrument::InstrumentIndex(0));
        instrument.data.l1.upsert_bids(vec![(dec!(100), dec!(1))]);
        instrument.data.l1.upsert_asks(vec![(dec!(101), dec!(1))]);
        // existing open order at 99
        use jackbot_execution::order::state::{ActiveOrderState, Open};
        use chrono::Utc;
        let existing = jackbot_execution::order::Order {
            key: jackbot_execution::order::OrderKey {
                exchange: jackbot_instrument::exchange::ExchangeIndex(0),
                instrument: jackbot_instrument::instrument::InstrumentIndex(0),
                strategy: jackbot_execution::order::id::StrategyId::new("test"),
                cid: jackbot_execution::order::id::ClientOrderId::new("cid"),
            },
            side: Side::Buy,
            price: dec!(99),
            quantity: dec!(1),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilCancelled { post_only: true },
            state: ActiveOrderState::Open(Open { id: jackbot_execution::order::id::OrderId::new("id"), time_exchange: Utc::now(), filled_quantity: dec!(0) }),
        };
        instrument.orders.0.insert(existing.key.cid.clone(), existing);
    }

    // book moves to 101
    {
        let instrument = state.instruments.instrument_index_mut(&jackbot_instrument::instrument::InstrumentIndex(0));
        instrument.data.l1.upsert_bids(vec![(dec!(101), dec!(1))]);
    }

    let strat = AlwaysMakerStrategy { id: jackbot_execution::order::id::StrategyId::new("test"), side: Side::Buy, quantity: dec!(1) };
    let (cancels, opens) = strat.generate_algo_orders(&state);
    assert_eq!(cancels.into_iter().count(), 1);
    let order = opens.into_iter().next().expect("order");
    assert_eq!(order.state.price, dec!(101));
}

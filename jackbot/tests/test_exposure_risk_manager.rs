use Jackbot::{
    engine::state::{EngineState, builder::EngineStateBuilder, global::DefaultGlobalData},
    risk::{RiskManager, exposure::{ExposureRiskManager, ExposureLimits, mitigation_actions, generate_dashboard}},
};
use jackbot_instrument::{
    Underlying,
    instrument::{Instrument, spec::{InstrumentSpec, InstrumentSpecPrice, InstrumentSpecQuantity, InstrumentSpecNotional, OrderQuantityUnits}},
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
    asset::AssetIndex,
};
use jackbot_execution::order::{
    id::{ClientOrderId, OrderId, StrategyId},
    request::{OrderRequestOpen, RequestOpen},
    OrderKey, OrderKind, TimeInForce,
};
use jackbot_execution::trade::{Trade, TradeId, AssetFees};
use jackbot_data::event::DataKind;
use jackbot::engine::state::instrument::data::DefaultInstrumentMarketData;
use jackbot::engine::state::instrument::filter::InstrumentFilter;
use chrono::{Utc, DateTime};
use rust_decimal_macros::dec;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[test]
fn test_exposure_risk_manager_blocks_excess_exposure() {
    let instruments = jackbot_instrument::index::IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .build();

    let mut state: EngineState<DefaultGlobalData, DefaultInstrumentMarketData> = EngineState::builder(
        &instruments,
        DefaultGlobalData::default(),
        DefaultInstrumentMarketData::default,
    )
    .time_engine_start(Utc::now())
    .build();

    let inst_key = InstrumentIndex(0);
    let mut inst_state = state.instruments.instrument_index_mut(&inst_key);
    inst_state.data.last_traded_price = Some(Jackbot::Timed::new(dec!(100), Utc::now()));

    let trade = Trade {
        id: TradeId::new("t1"),
        order_id: OrderId::new("o1"),
        instrument: inst_key,
        strategy: StrategyId::new("s1"),
        time_exchange: Utc::now(),
        side: jackbot_instrument::Side::Buy,
        price: dec!(100),
        quantity: dec!(4),
        fees: AssetFees::quote_fees(dec!(0)),
    };
    inst_state.update_from_trade(&trade);
    drop(inst_state);

    let limits = ExposureLimits {
        max_notional_per_underlying: dec!(400),
        max_drawdown_percent: dec!(1),
        correlation_limits: HashMap::new(),
    };
    let risk: ExposureRiskManager<EngineState<_, _>> = ExposureRiskManager { limits, phantom: std::marker::PhantomData };

    let open = OrderRequestOpen {
        key: OrderKey {
            exchange: ExchangeIndex(0),
            instrument: inst_key,
            strategy: StrategyId::new("s1"),
            cid: ClientOrderId::new("c1"),
        },
        state: RequestOpen {
            side: jackbot_instrument::Side::Buy,
            price: dec!(100),
            quantity: dec!(1),
            kind: OrderKind::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
        },
    };

    let (_approved_cancels, approved_opens, _refused_cancels, refused_opens) =
        risk.check(&state, std::iter::empty::<OrderRequestOpen>(), vec![open]);
    let approved: Vec<_> = approved_opens.into_iter().collect();
    let refused: Vec<_> = refused_opens.into_iter().collect();
    assert!(approved.is_empty());
    assert_eq!(refused.len(), 1);
}

#[test]
fn test_mitigation_actions_drawdown() {
    let instruments = jackbot_instrument::index::IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .build();

    let mut state: EngineState<DefaultGlobalData, DefaultInstrumentMarketData> = EngineState::builder(
        &instruments,
        DefaultGlobalData::default(),
        DefaultInstrumentMarketData::default,
    )
    .time_engine_start(Utc::now())
    .build();

    let inst_key = InstrumentIndex(0);
    let mut inst_state = state.instruments.instrument_index_mut(&inst_key);
    inst_state.data.last_traded_price = Some(Jackbot::Timed::new(dec!(100), Utc::now()));

    let trade = Trade {
        id: TradeId::new("t1"),
        order_id: OrderId::new("o1"),
        instrument: inst_key,
        strategy: StrategyId::new("s1"),
        time_exchange: Utc::now(),
        side: jackbot_instrument::Side::Buy,
        price: dec!(100),
        quantity: dec!(4),
        fees: AssetFees::quote_fees(dec!(0)),
    };
    inst_state.update_from_trade(&trade);
    inst_state.data.last_traded_price = Some(Jackbot::Timed::new(dec!(50), Utc::now()));
    inst_state.position.current.as_mut().unwrap().update_pnl_unrealised(dec!(50));
    drop(inst_state);

    let limits = ExposureLimits {
        max_notional_per_underlying: dec!(1000),
        max_drawdown_percent: dec!(0.2),
        correlation_limits: HashMap::new(),
    };

    let actions = mitigation_actions(&limits, &state);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Jackbot::engine::command::Command::ClosePositions(filter) => match filter {
            InstrumentFilter::Instruments(list) => assert_eq!(list.len(), 1),
            _ => panic!("unexpected filter"),
        },
        _ => panic!("unexpected command"),
    }
}

#[test]
fn test_mitigation_actions_exposure_partial_close() {
    let instruments = jackbot_instrument::index::IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .build();

    let mut state: EngineState<DefaultGlobalData, DefaultInstrumentMarketData> = EngineState::builder(
        &instruments,
        DefaultGlobalData::default(),
        DefaultInstrumentMarketData::default,
    )
    .time_engine_start(Utc::now())
    .build();

    let inst_key = InstrumentIndex(0);
    let mut inst_state = state.instruments.instrument_index_mut(&inst_key);
    inst_state.data.last_traded_price = Some(Jackbot::Timed::new(dec!(100), Utc::now()));

    let trade = Trade {
        id: TradeId::new("t1"),
        order_id: OrderId::new("o1"),
        instrument: inst_key,
        strategy: StrategyId::new("s1"),
        time_exchange: Utc::now(),
        side: jackbot_instrument::Side::Buy,
        price: dec!(100),
        quantity: dec!(5),
        fees: AssetFees::quote_fees(dec!(0)),
    };
    inst_state.update_from_trade(&trade);
    drop(inst_state);

    let limits = ExposureLimits {
        max_notional_per_underlying: dec!(400),
        max_drawdown_percent: dec!(1),
        correlation_limits: HashMap::new(),
    };

    let actions = mitigation_actions(&limits, &state);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Jackbot::engine::command::Command::SendOpenRequests(req) => {
            let order = match req {
                jackbot_integration::collection::one_or_many::OneOrMany::One(o) => o,
                _ => panic!("expected one order"),
            };
            assert_eq!(order.state.quantity, dec!(1));
            assert_eq!(order.state.side, jackbot_instrument::Side::Sell);
        }
        _ => panic!("unexpected command"),
    }
}

#[test]
fn test_mitigation_actions_correlation_hedge() {
    let instruments = jackbot_instrument::index::IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_busd",
            "BTCBUSD",
            Underlying::new("btc", "busd"),
            None,
        ))
        .build();

    let mut state: EngineState<DefaultGlobalData, DefaultInstrumentMarketData> = EngineState::builder(
        &instruments,
        DefaultGlobalData::default(),
        DefaultInstrumentMarketData::default,
    )
    .time_engine_start(Utc::now())
    .build();

    let key_a = InstrumentIndex(0);
    let key_b = InstrumentIndex(1);

    let mut state_a = state.instruments.instrument_index_mut(&key_a);
    state_a.data.last_traded_price = Some(Jackbot::Timed::new(dec!(10), Utc::now()));
    let trade_a = Trade {
        id: TradeId::new("t1"),
        order_id: OrderId::new("o1"),
        instrument: key_a,
        strategy: StrategyId::new("s1"),
        time_exchange: Utc::now(),
        side: jackbot_instrument::Side::Buy,
        price: dec!(10),
        quantity: dec!(3),
        fees: AssetFees::quote_fees(dec!(0)),
    };
    state_a.update_from_trade(&trade_a);
    drop(state_a);

    let mut state_b = state.instruments.instrument_index_mut(&key_b);
    state_b.data.last_traded_price = Some(Jackbot::Timed::new(dec!(10), Utc::now()));
    let trade_b = Trade {
        id: TradeId::new("t2"),
        order_id: OrderId::new("o2"),
        instrument: key_b,
        strategy: StrategyId::new("s1"),
        time_exchange: Utc::now(),
        side: jackbot_instrument::Side::Buy,
        price: dec!(10),
        quantity: dec!(3),
        fees: AssetFees::quote_fees(dec!(0)),
    };
    state_b.update_from_trade(&trade_b);
    drop(state_b);

    let mut corr = HashMap::new();
    corr.insert((key_a, key_b), dec!(50));

    let limits = ExposureLimits {
        max_notional_per_underlying: dec!(1000),
        max_drawdown_percent: dec!(1),
        correlation_limits: corr,
    };

    let actions = mitigation_actions(&limits, &state);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Jackbot::engine::command::Command::SendOpenRequests(req) => {
            match req {
                jackbot_integration::collection::one_or_many::OneOrMany::One(o) => {
                    assert_eq!(o.state.quantity, dec!(1));
                }
                _ => panic!("expected one order"),
            }
        }
        _ => panic!("unexpected command"),
    }
}

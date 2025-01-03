use barter::{
    engine::{
        action::{
            generate_algo_orders::GenerateAlgoOrdersOutput,
            send_requests::{SendCancelsAndOpensOutput, SendRequestsOutput},
        },
        audit::Audit,
        clock::HistoricalClock,
        execution_tx::MultiExchangeTxMap,
        state::{
            asset::AssetStates,
            connectivity::Health,
            instrument::{
                filter::InstrumentFilter,
                market_data::{DefaultMarketData, MarketDataState},
            },
            order::manager::OrderManager,
            trading::TradingState,
            EngineState,
        },
        Engine, EngineOutput, Processor,
    },
    execution::{request::ExecutionRequest, AccountStreamEvent},
    risk::{DefaultRiskManager, DefaultRiskManagerState},
    strategy::{
        algo::AlgoStrategy,
        close_positions::{close_open_positions_with_market_orders, ClosePositionsStrategy},
        on_disconnect::OnDisconnectStrategy,
        on_trading_disabled::OnTradingDisabled,
        DefaultStrategyState,
    },
    test_utils::time_plus_days,
    EngineEvent, Sequence,
};
use barter_data::{
    event::{DataKind, MarketEvent},
    streams::consumer::MarketStreamEvent,
    subscription::trade::PublicTrade,
};
use barter_execution::{
    balance::{AssetBalance, Balance},
    order::{ClientOrderId, Order, OrderKind, RequestCancel, RequestOpen, StrategyId, TimeInForce},
    AccountEvent, AccountEventKind, AccountSnapshot,
};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    index::IndexedInstruments,
    instrument::{
        kind::InstrumentKind,
        spec::{
            InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            OrderQuantityUnits,
        },
        Instrument, InstrumentIndex,
    },
    Side, Underlying,
};
use barter_integration::{
    channel::{mpsc_unbounded, UnboundedTx},
    collection::none_one_or_many::NoneOneOrMany,
};
use chrono::{DateTime, Utc};
use fnv::FnvHashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

const STARTING_TIMESTAMP: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
const RISK_FREE_RETURN: Decimal = dec!(0.05);
const STARTING_BALANCE_USDT: Balance = Balance {
    total: dec!(10_000.0),
    free: dec!(10_000.0),
};
const STARTING_BALANCE_BTC: Balance = Balance {
    total: dec!(0.1),
    free: dec!(0.1),
};
const STARTING_BALANCE_ETH: Balance = Balance {
    total: dec!(1.0),
    free: dec!(1.0),
};

// Todo:
//  - do we want to test Sequence?
//  - Consider adding "no algo orders generated if none were generated" <- this is important, it's wasteful
//  - Consider engine "process_with_audit" to enable testing of audit tick generation & sequence
//  - Could add Uuid generator to Instrument :thinking
//  - Add utils to determine if orders are in flight eg/ orders.filtered() or orders.num_in_flight

#[test]
fn test_engine_process_engine_event() {
    let (execution_tx, mut execution_rx) = mpsc_unbounded();

    let mut engine = build_engine(TradingState::Disabled, execution_tx);
    assert_eq!(engine.meta.sequence, Sequence(0));
    assert_eq!(engine.state.connectivity.global, Health::Reconnecting);

    // Simulate AccountSnapshot from ExecutionManager::init
    let snapshot = account_event_snapshot(&engine.state.assets);
    let output = engine.process(snapshot.clone());
    assert_eq!(engine.state.connectivity.global, Health::Reconnecting);
    assert_eq!(output, Audit::process(snapshot));

    // Process 1st MarketEvent for btc_usdt
    let output = engine.process(market_event_trade(1, 0, 10_000.0));
    assert_eq!(engine.state.connectivity.global, Health::Healthy);
    assert_eq!(output, Audit::process(market_event_trade(1, 0, 10_000.0)));

    // Process 1st MarketEvent for eth_btc
    let output = engine.process(market_event_trade(1, 1, 0.1));
    assert_eq!(output, Audit::process(market_event_trade(1, 1, 0.1)));

    // TradingState::Enabled -> expect BuyAndHoldStrategy to open Buy orders
    let output = engine.process(EngineEvent::TradingStateUpdate(TradingState::Enabled));
    assert_eq!(
        output,
        Audit::process_with_output(
            EngineEvent::TradingStateUpdate(TradingState::Enabled),
            EngineOutput::AlgoOrders(GenerateAlgoOrdersOutput {
                cancels_and_opens: SendCancelsAndOpensOutput {
                    cancels: SendRequestsOutput::default(),
                    opens: SendRequestsOutput {
                        sent: NoneOneOrMany::Many(vec![
                            Order {
                                exchange: ExchangeIndex(0),
                                instrument: InstrumentIndex(0),
                                strategy: StrategyId::new("TestBuyAndHoldStrategy"),
                                cid: ClientOrderId::new(Uuid::nil()),
                                side: Side::Buy,
                                state: RequestOpen {
                                    kind: OrderKind::Market,
                                    time_in_force: TimeInForce::ImmediateOrCancel,
                                    price: dec!(10_000),
                                    quantity: dec!(0.00001),
                                },
                            },
                            Order {
                                exchange: ExchangeIndex(0),
                                instrument: InstrumentIndex(1),
                                strategy: StrategyId::new("TestBuyAndHoldStrategy"),
                                cid: ClientOrderId::new(Uuid::max()),
                                side: Side::Buy,
                                state: RequestOpen {
                                    kind: OrderKind::Market,
                                    time_in_force: TimeInForce::ImmediateOrCancel,
                                    price: dec!(0.1),
                                    quantity: dec!(0.0001),
                                },
                            },
                        ]),
                        errors: NoneOneOrMany::None,
                    },
                },
                ..Default::default()
            })
        )
    );

    // Ensure ExecutionRequests were sent to ExecutionManager
    assert_eq!(
        execution_rx.next().unwrap(),
        ExecutionRequest::Open(Order {
            exchange: ExchangeIndex(0),
            instrument: InstrumentIndex(0),
            strategy: StrategyId::new("TestBuyAndHoldStrategy"),
            cid: ClientOrderId::new(Uuid::nil()),
            side: Side::Buy,
            state: RequestOpen {
                kind: OrderKind::Market,
                time_in_force: TimeInForce::ImmediateOrCancel,
                price: dec!(10_000),
                quantity: dec!(0.00001),
            },
        })
    );
    assert_eq!(
        execution_rx.next().unwrap(),
        ExecutionRequest::Open(Order {
            exchange: ExchangeIndex(0),
            instrument: InstrumentIndex(1),
            strategy: StrategyId::new("TestBuyAndHoldStrategy"),
            cid: ClientOrderId::new(Uuid::max()),
            side: Side::Buy,
            state: RequestOpen {
                kind: OrderKind::Market,
                time_in_force: TimeInForce::ImmediateOrCancel,
                price: dec!(0.1),
                quantity: dec!(0.0001),
            },
        })
    );

    // Process 2nd MarketEvent for btc_usdt
    let output = engine.process(market_event_trade(2, 0, 20_000.0));
    assert_eq!(
        output,
        Audit::process_with_output(
            market_event_trade(2, 0, 20_000.0),
            EngineOutput::AlgoOrders(GenerateAlgoOrdersOutput::default())
        )
    );

    // Process 2nd MarketEvent for eth_btc
    let output = engine.process(market_event_trade(2, 1, 0.05));
    assert_eq!(
        output,
        Audit::process_with_output(
            market_event_trade(2, 1, 0.05),
            EngineOutput::AlgoOrders(GenerateAlgoOrdersOutput::default())
        )
    );

    // Issue CloseAllPositions Command

    // let output = engine.process()
}

struct TestBuyAndHoldStrategy {
    id: StrategyId,
}

impl AlgoStrategy for TestBuyAndHoldStrategy {
    type State = EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>;

    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestOpen>>,
    ) {
        let opens = state.instruments.instruments().filter_map(|state| {
            // Don't open more if we have a Position already
            if state.position.is_some() {
                return None;
            }

            // Todo: Don't open more if there is already requests in flight
            // if state
            //     .orders
            //     .orders()
            //     .map(|x| )

            // Don't open if there is no market data price available
            let price = state.market.price()?;

            let cid = if state.key == InstrumentIndex(0) {
                Uuid::nil()
            } else {
                Uuid::max()
            };

            // Generate Market order to buy the minimum allowed quantity
            Some(Order {
                exchange: state.instrument.exchange,
                instrument: state.key,
                strategy: self.id.clone(),
                cid: ClientOrderId::new(cid),
                side: Side::Buy,
                state: RequestOpen {
                    kind: OrderKind::Market,
                    time_in_force: TimeInForce::ImmediateOrCancel,
                    price,
                    quantity: state.instrument.spec.unwrap().quantity.min,
                },
            })
        });

        (std::iter::empty(), opens)
    }
}

impl ClosePositionsStrategy for TestBuyAndHoldStrategy {
    type State = EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>;

    fn close_positions_requests<'a>(
        &'a self,
        state: &'a Self::State,
        filter: &'a InstrumentFilter<ExchangeIndex, AssetIndex, InstrumentIndex>,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestCancel>> + 'a,
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestOpen>> + 'a,
    )
    where
        ExchangeIndex: 'a,
        AssetIndex: 'a,
        InstrumentIndex: 'a,
    {
        close_open_positions_with_market_orders(&self.id, state, filter)
    }
}

impl
    OnDisconnectStrategy<
        HistoricalClock,
        EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
        MultiExchangeTxMap<UnboundedTx<ExecutionRequest>>,
        DefaultRiskManager<
            EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
        >,
    > for TestBuyAndHoldStrategy
{
    type OnDisconnect = ();

    fn on_disconnect(
        _: &mut Engine<
            HistoricalClock,
            EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
            MultiExchangeTxMap<UnboundedTx<ExecutionRequest>>,
            Self,
            DefaultRiskManager<
                EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
            >,
        >,
        _: ExchangeId,
    ) -> Self::OnDisconnect {
    }
}

impl
    OnTradingDisabled<
        HistoricalClock,
        EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
        MultiExchangeTxMap<UnboundedTx<ExecutionRequest>>,
        DefaultRiskManager<
            EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
        >,
    > for TestBuyAndHoldStrategy
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _: &mut Engine<
            HistoricalClock,
            EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
            MultiExchangeTxMap<UnboundedTx<ExecutionRequest>>,
            Self,
            DefaultRiskManager<
                EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
            >,
        >,
    ) -> Self::OnTradingDisabled {
    }
}

fn build_engine(
    trading_state: TradingState,
    execution_tx: UnboundedTx<ExecutionRequest>,
) -> Engine<
    HistoricalClock,
    EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
    MultiExchangeTxMap<UnboundedTx<ExecutionRequest>>,
    TestBuyAndHoldStrategy,
    DefaultRiskManager<
        EngineState<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>,
    >,
> {
    let instruments = IndexedInstruments::builder()
        .add_instrument(Instrument::new(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            InstrumentKind::Spot,
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(
                    OrderQuantityUnits::Quote,
                    dec!(0.00001),
                    dec!(0.00001),
                ),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::new(
            ExchangeId::BinanceSpot,
            "binance_spot_eth_btc",
            "ETHBTC",
            Underlying::new("eth", "btc"),
            InstrumentKind::Spot,
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.00001), dec!(0.00001)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.0001), dec!(0.0001)),
                InstrumentSpecNotional::new(dec!(0.0001)),
            )),
        ))
        .build();

    let clock = HistoricalClock::new(STARTING_TIMESTAMP);

    let state =
        EngineState::<DefaultMarketData, DefaultStrategyState, DefaultRiskManagerState>::builder(
            &instruments,
        )
        .time_engine_start(STARTING_TIMESTAMP)
        .trading_state(trading_state)
        .balances([
            (ExchangeId::BinanceSpot, "usdt", STARTING_BALANCE_USDT),
            (ExchangeId::BinanceSpot, "btc", STARTING_BALANCE_BTC),
            (ExchangeId::BinanceSpot, "eth", STARTING_BALANCE_ETH),
        ])
        .build();

    let initial_account = FnvHashMap::from(&state);
    assert_eq!(initial_account.len(), 1);

    let execution_txs =
        MultiExchangeTxMap::from_iter([(ExchangeId::BinanceSpot, Some(execution_tx))]);

    Engine::new(
        clock,
        state,
        execution_txs,
        TestBuyAndHoldStrategy {
            id: StrategyId::new("TestBuyAndHoldStrategy"),
        },
        DefaultRiskManager::default(),
    )
}

fn account_event_snapshot(assets: &AssetStates) -> EngineEvent<DataKind> {
    EngineEvent::Account(AccountStreamEvent::Item(AccountEvent {
        exchange: ExchangeIndex(0),
        kind: AccountEventKind::Snapshot(AccountSnapshot {
            exchange: ExchangeIndex(0),
            balances: assets
                .0
                .iter()
                .enumerate()
                .map(|(index, (_, state))| AssetBalance {
                    asset: AssetIndex(index),
                    balance: state.balance.unwrap().value,
                    time_exchange: state.balance.unwrap().time,
                })
                .collect(),
            instruments: vec![],
        }),
    }))
}

fn market_event_trade(time_plus: u64, instrument: usize, price: f64) -> EngineEvent<DataKind> {
    EngineEvent::Market(MarketStreamEvent::Item(MarketEvent {
        time_exchange: time_plus_days(STARTING_TIMESTAMP, time_plus),
        time_received: time_plus_days(STARTING_TIMESTAMP, time_plus),
        exchange: ExchangeId::BinanceSpot,
        instrument: InstrumentIndex(instrument),
        kind: DataKind::Trade(PublicTrade {
            id: time_plus.to_string(),
            price,
            amount: 1.0,
            side: Side::Buy,
        }),
    }))
}

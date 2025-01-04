use barter::{
    engine::{
        action::{
            generate_algo_orders::GenerateAlgoOrdersOutput,
            send_requests::{SendCancelsAndOpensOutput, SendRequestsOutput},
            ActionOutput,
        },
        audit::EngineAudit,
        clock::HistoricalClock,
        command::Command,
        execution_tx::MultiExchangeTxMap,
        process_with_audit,
        state::{
            asset::AssetStates,
            connectivity::Health,
            instrument::{
                filter::InstrumentFilter,
                market_data::{DefaultMarketData, MarketDataState},
            },
            position::PositionExited,
            trading::TradingState,
            EngineState,
        },
        Engine, EngineOutput,
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
    EngineEvent, Sequence, Timed,
};
use barter_data::{
    event::{DataKind, MarketEvent},
    streams::consumer::MarketStreamEvent,
    subscription::trade::PublicTrade,
};
use barter_execution::{
    balance::{AssetBalance, Balance},
    order::{
        ClientOrderId, Open, Order, OrderId, OrderKind, RequestCancel, RequestOpen, StrategyId,
        TimeInForce,
    },
    trade::{AssetFees, Trade, TradeId},
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
    collection::{none_one_or_many::NoneOneOrMany, one_or_many::OneOrMany},
    snapshot::Snapshot,
};
use chrono::{DateTime, Utc};
use fnv::FnvHashMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

const STARTING_TIMESTAMP: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
const RISK_FREE_RETURN: Decimal = dec!(0.05);
const STARTING_BALANCE_USDT: Balance = Balance {
    total: dec!(40_000.0),
    free: dec!(40_000.0),
};
const STARTING_BALANCE_BTC: Balance = Balance {
    total: dec!(1.0),
    free: dec!(1.0),
};
const STARTING_BALANCE_ETH: Balance = Balance {
    total: dec!(10.0),
    free: dec!(10.0),
};
const QUOTE_FEES_PERCENT: f64 = 0.1; // 10%

#[test]
fn test_engine_process_engine_event_with_audit() {
    let (execution_tx, mut execution_rx) = mpsc_unbounded();

    let mut engine = build_engine(TradingState::Disabled, execution_tx);
    assert_eq!(engine.meta.sequence, Sequence(0));
    assert_eq!(engine.state.connectivity.global, Health::Reconnecting);

    // Simulate AccountSnapshot from ExecutionManager::init
    let event = account_event_snapshot(&engine.state.assets);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(0));
    assert_eq!(audit.event, EngineAudit::process(event));
    assert_eq!(engine.state.connectivity.global, Health::Reconnecting);

    // Process 1st MarketEvent for btc_usdt
    let event = market_event_trade(1, 0, 10_000.0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(1));
    assert_eq!(audit.event, EngineAudit::process(event));
    assert_eq!(engine.state.connectivity.global, Health::Healthy);

    // Process 1st MarketEvent for eth_btc
    let event = market_event_trade(1, 1, 0.1);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(2));
    assert_eq!(audit.event, EngineAudit::process(event));

    // TradingState::Enabled -> expect BuyAndHoldStrategy to open Buy orders
    let event = EngineEvent::TradingStateUpdate(TradingState::Enabled);
    let audit = process_with_audit(&mut engine, event);
    assert_eq!(audit.context.sequence, Sequence(3));
    let btc_usdt_buy_order = Order {
        exchange: ExchangeIndex(0),
        instrument: InstrumentIndex(0),
        strategy: strategy_id(),
        cid: gen_cid(0),
        side: Side::Buy,
        state: RequestOpen {
            kind: OrderKind::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
            price: dec!(10_000),
            quantity: dec!(1),
        },
    };
    let eth_btc_buy_order = Order {
        exchange: ExchangeIndex(0),
        instrument: InstrumentIndex(1),
        strategy: strategy_id(),
        cid: gen_cid(1),
        side: Side::Buy,
        state: RequestOpen {
            kind: OrderKind::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
            price: dec!(0.1),
            quantity: dec!(1),
        },
    };
    assert_eq!(
        audit.event,
        EngineAudit::process_with_output(
            EngineEvent::TradingStateUpdate(TradingState::Enabled),
            EngineOutput::AlgoOrders(GenerateAlgoOrdersOutput {
                cancels_and_opens: SendCancelsAndOpensOutput {
                    cancels: SendRequestsOutput::default(),
                    opens: SendRequestsOutput {
                        sent: NoneOneOrMany::Many(vec![
                            btc_usdt_buy_order.clone(),
                            eth_btc_buy_order.clone(),
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
        ExecutionRequest::Open(btc_usdt_buy_order)
    );
    assert_eq!(
        execution_rx.next().unwrap(),
        ExecutionRequest::Open(eth_btc_buy_order)
    );

    // TradingState::Disabled
    let event = EngineEvent::TradingStateUpdate(TradingState::Disabled);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(4));
    assert_eq!(
        audit.event,
        EngineAudit::process_with_output(
            event,
            EngineOutput::OnTradingDisabled(OnTradingDisabledOutput)
        )
    );

    // Simulate OpenOrder response for Sequence(3) btc_usdt_buy_order
    let event = account_event_order_response(0, 2, Side::Buy, 10_000.0, 1.0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(5));
    assert_eq!(audit.event, EngineAudit::process(event));
    assert!(engine
        .state
        .instruments
        .instrument_index(&InstrumentIndex(0))
        .orders
        .0
        .is_empty());

    // Simulate Trade update for Sequence(3) btc_usdt_buy_order (fees 10% -> 1000usdt)
    let event = account_event_trade(0, 2, Side::Buy, 10_000.0, 1.0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(6));
    assert_eq!(audit.event, EngineAudit::process(event));

    // Simulate Balance update for Sequence(3) btc_usdt_buy_order, AssetIndex(2)/usdt reduction
    let event = account_event_balance(2, 2, 9_000.0); // 10k - 10% fees
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(7));
    assert_eq!(audit.event, EngineAudit::process(event));
    assert_eq!(
        engine
            .state
            .assets
            .asset_index(&AssetIndex(2))
            .balance
            .unwrap(),
        Timed::new(
            Balance::new(dec!(9_000.0), dec!(9_000.0)),
            time_plus_days(STARTING_TIMESTAMP, 2)
        )
    );

    // Simulate OpenOrder response for Sequence(3) eth_btc_buy_order
    let event = account_event_order_response(1, 2, Side::Buy, 0.1, 1.0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(8));
    assert_eq!(audit.event, EngineAudit::process(event));
    assert!(engine
        .state
        .instruments
        .instrument_index(&InstrumentIndex(1))
        .orders
        .0
        .is_empty());

    // Simulate Trade update for Sequence(3) eth_btc_buy_order (fees 10% -> 0.01btc)
    let event = account_event_trade(1, 2, Side::Buy, 0.1, 1.0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(9));
    assert_eq!(audit.event, EngineAudit::process(event));

    // Simulate Balance update for Sequence(3) eth_btc_buy_order, AssetIndex(0)/btc reduction
    let event = account_event_balance(0, 2, 0.99); // 1btc - 10% fees
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(10));
    assert_eq!(audit.event, EngineAudit::process(event));
    assert_eq!(
        engine
            .state
            .assets
            .asset_index(&AssetIndex(0))
            .balance
            .unwrap(),
        Timed::new(
            Balance::new(dec!(0.99), dec!(0.99)),
            time_plus_days(STARTING_TIMESTAMP, 2)
        )
    );

    // Process 2nd MarketEvent for btc_usdt
    let event = market_event_trade(2, 0, 20_000.0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(11));
    assert_eq!(audit.event, EngineAudit::process(event));

    // Process 2nd MarketEvent for eth_btc
    let event = market_event_trade(2, 1, 0.05);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(12));
    assert_eq!(audit.event, EngineAudit::process(event));

    // Send ClosePositionsCommand for btc_usdt
    let event = command_close_position(0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(13));
    let btc_usdt_sell_order = Order {
        exchange: ExchangeIndex(0),
        instrument: InstrumentIndex(0),
        strategy: strategy_id(),
        cid: gen_cid(0),
        side: Side::Sell,
        state: RequestOpen {
            kind: OrderKind::Market,
            time_in_force: TimeInForce::ImmediateOrCancel,
            price: dec!(20_000),
            quantity: dec!(1),
        },
    };
    assert_eq!(
        audit.event,
        EngineAudit::process_with_output(
            event,
            EngineOutput::Commanded(ActionOutput::ClosePositions(SendCancelsAndOpensOutput {
                cancels: SendRequestsOutput::default(),
                opens: SendRequestsOutput {
                    sent: NoneOneOrMany::One(btc_usdt_sell_order.clone()),
                    errors: NoneOneOrMany::None,
                },
            }))
        )
    );

    // Ensure ClosePositions ExecutionRequest was sent to ExecutionManager
    assert_eq!(
        execution_rx.next().unwrap(),
        ExecutionRequest::Open(btc_usdt_sell_order)
    );

    // Simulate OpenOrder response for Sequence(13) btc_usdt_sell_order
    let event = account_event_order_response(0, 3, Side::Sell, 20_000.0, 1.0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(14));
    assert_eq!(audit.event, EngineAudit::process(event));
    assert!(engine
        .state
        .instruments
        .instrument_index(&InstrumentIndex(0))
        .orders
        .0
        .is_empty());

    // Simulate Balance update for Sequence(13) btc_usdt_sell_order, AssetIndex(2)/usdt increase
    let event = account_event_balance(2, 3, 27_000.0); // 9k + 20k - 10% fees
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(15));
    assert_eq!(audit.event, EngineAudit::process(event));
    assert_eq!(
        engine
            .state
            .assets
            .asset_index(&AssetIndex(2))
            .balance
            .unwrap(),
        Timed::new(
            Balance::new(dec!(27_000.0), dec!(27_000.0)),
            time_plus_days(STARTING_TIMESTAMP, 3)
        )
    );

    // Simulate Trade update for Sequence(13) btc_usdt_sell_order (fees 10% -> 2000usdt)
    let event = account_event_trade(0, 3, Side::Sell, 20_000.0, 1.0);
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(16));
    assert_eq!(
        audit.event,
        EngineAudit::process_with_output(
            event,
            PositionExited {
                instrument: InstrumentIndex(0),
                side: Side::Buy,
                price_entry_average: dec!(10_000.0),
                quantity_abs_max: dec!(1.0),
                pnl_realised: dec!(7000.0), // (-10k entry - 1k fees)+(20k exit - 2k fees) = 7k
                fees_enter: AssetFees::quote_fees(dec!(1_000.0)),
                fees_exit: AssetFees::quote_fees(dec!(2_000.0)),
                time_enter: time_plus_days(STARTING_TIMESTAMP, 2),
                time_exit: time_plus_days(STARTING_TIMESTAMP, 3),
                trades: vec![gen_trade_id(0), gen_trade_id(0)],
            }
        )
    );

    // Simulate exchange disconnection
    let event = EngineEvent::Market(MarketStreamEvent::Reconnecting(ExchangeId::BinanceSpot));
    let audit = process_with_audit(&mut engine, event.clone());
    assert_eq!(audit.context.sequence, Sequence(17));
    assert_eq!(
        audit.event,
        EngineAudit::process_with_output(event, EngineOutput::MarketDisconnect(OnDisconnectOutput))
    );
    assert_eq!(engine.state.connectivity.global, Health::Reconnecting);
    assert_eq!(
        engine
            .state
            .connectivity
            .connectivity(&ExchangeId::BinanceSpot)
            .market_data,
        Health::Reconnecting
    );
    assert_eq!(
        engine
            .state
            .connectivity
            .connectivity(&ExchangeId::BinanceSpot)
            .account,
        Health::Healthy
    );

    // Todo: Command::OpenOrder LIMIT to increase eth_btc position & run through some order states
    // Todo: TradingSummaryGenerator
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

            // Don't open more orders if there are already some InFlight
            if !state.orders.0.is_empty() {
                return None;
            }

            // Don't open if there is no market data price available
            let price = state.market.price()?;

            // Generate Market order to buy the minimum allowed quantity
            Some(Order {
                exchange: state.instrument.exchange,
                instrument: state.key,
                strategy: self.id.clone(),
                cid: gen_cid(state.key.index()),
                side: Side::Buy,
                state: RequestOpen {
                    kind: OrderKind::Market,
                    time_in_force: TimeInForce::ImmediateOrCancel,
                    price,
                    quantity: dec!(1),
                },
            })
        });

        (std::iter::empty(), opens)
    }
}

fn strategy_id() -> StrategyId {
    StrategyId::new("TestBuyAndHoldStrategy")
}

fn gen_cid(instrument: usize) -> ClientOrderId {
    ClientOrderId::new(InstrumentIndex(instrument).to_string())
}

fn gen_trade_id(instrument: usize) -> TradeId {
    TradeId::new(InstrumentIndex(instrument).to_string())
}

fn gen_order_id(instrument: usize) -> OrderId {
    OrderId::new(InstrumentIndex(instrument).to_string())
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

#[derive(Debug, PartialEq)]
struct OnDisconnectOutput;
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
    type OnDisconnect = OnDisconnectOutput;

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
        OnDisconnectOutput
    }
}

#[derive(Debug, PartialEq)]
struct OnTradingDisabledOutput;
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
    type OnTradingDisabled = OnTradingDisabledOutput;

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
        OnTradingDisabledOutput
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
        TestBuyAndHoldStrategy { id: strategy_id() },
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

fn account_event_order_response(
    instrument: usize,
    time_plus: u64,
    side: Side,
    price: f64,
    quantity: f64,
) -> EngineEvent<DataKind> {
    EngineEvent::Account(AccountStreamEvent::Item(AccountEvent {
        exchange: ExchangeIndex(0),
        kind: AccountEventKind::OrderOpened(Order {
            exchange: ExchangeIndex(0),
            instrument: InstrumentIndex(instrument),
            strategy: strategy_id(),
            cid: gen_cid(instrument),
            side,
            state: Ok(Open {
                id: OrderId::new(instrument.to_string()),
                time_exchange: time_plus_days(STARTING_TIMESTAMP, time_plus),
                price: Decimal::try_from(price).unwrap(),
                quantity: Decimal::try_from(quantity).unwrap(),
                filled_quantity: Decimal::try_from(quantity).unwrap(),
            }),
        }),
    }))
}

fn account_event_balance(asset: usize, time_plus: u64, value: f64) -> EngineEvent<DataKind> {
    EngineEvent::Account(AccountStreamEvent::Item(AccountEvent {
        exchange: ExchangeIndex(0),
        kind: AccountEventKind::BalanceSnapshot(Snapshot(AssetBalance {
            asset: AssetIndex(asset),
            balance: Balance::new(
                Decimal::try_from(value).unwrap(),
                Decimal::try_from(value).unwrap(),
            ),
            time_exchange: time_plus_days(STARTING_TIMESTAMP, time_plus),
        })),
    }))
}

fn account_event_trade(
    instrument: usize,
    time_plus: u64,
    side: Side,
    price: f64,
    quantity: f64,
) -> EngineEvent<DataKind> {
    EngineEvent::Account(AccountStreamEvent::Item(AccountEvent {
        exchange: ExchangeIndex(0),
        kind: AccountEventKind::Trade(Trade {
            id: gen_trade_id(instrument),
            order_id: gen_order_id(instrument),
            instrument: InstrumentIndex(instrument),
            strategy: strategy_id(),
            time_exchange: time_plus_days(STARTING_TIMESTAMP, time_plus),
            side,
            price: Decimal::try_from(price).unwrap(),
            quantity: Decimal::try_from(quantity).unwrap(),
            fees: AssetFees::quote_fees(
                Decimal::try_from(price * quantity * QUOTE_FEES_PERCENT).unwrap(),
            ),
        }),
    }))
}

fn command_close_position(instrument: usize) -> EngineEvent<DataKind> {
    EngineEvent::Command(Command::ClosePositions(InstrumentFilter::Instruments(
        OneOrMany::One(InstrumentIndex(instrument)),
    )))
}

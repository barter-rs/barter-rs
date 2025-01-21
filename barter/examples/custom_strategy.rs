use barter::{
    engine::{
        audit::EngineAudit,
        clock::{EngineClock, LiveClock},
        command::Command,
        run,
        state::{
            self,
            instrument::{
                filter::InstrumentFilter,
                market_data::{DefaultMarketData, MarketDataState},
            },
            order::manager::OrderManager,
            trading::TradingState,
        },
        Engine, Processor,
    },
    execution::builder::ExecutionBuilder,
    logging::init_logging,
    risk::{DefaultRiskManager, DefaultRiskManagerState},
    statistic::time::Daily,
    strategy::{
        algo::AlgoStrategy, close_positions::ClosePositionsStrategy,
        on_disconnect::OnDisconnectStrategy, on_trading_disabled::OnTradingDisabled,
    },
    EngineEvent,
};
use barter_data::{
    event::MarketEvent,
    streams::{
        builder::dynamic::indexed::init_indexed_multi_exchange_market_stream,
        reconnect::stream::ReconnectingStream,
    },
    subscription::SubKind,
};
use barter_execution::{
    balance::Balance,
    client::mock::MockExecutionConfig,
    order::{
        ClientOrderId, InternalOrderState, Order, OrderKind, RequestCancel, RequestOpen,
        StrategyId, TimeInForce,
    },
    trade::Trade,
    AccountEvent, AccountEventKind,
};
use barter_instrument::{
    asset::{AssetIndex, QuoteAsset},
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
use barter_integration::channel::{mpsc_unbounded, ChannelTxDroppable, Tx};
use fnv::FnvHashMap;
use futures::StreamExt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{debug, warn};

const EXCHANGE: ExchangeId = ExchangeId::BinanceSpot;
const RISK_FREE_RETURN: Decimal = dec!(0.05);
const MOCK_EXCHANGE_ROUND_TRIP_LATENCY_MS: u64 = 100;
const MOCK_EXCHANGE_FEES_PERCENT: Decimal = dec!(0.05);
const STARTING_BALANCE_USDT: Balance = Balance {
    total: dec!(10_000.0),
    free: dec!(10_000.0),
};
const STARTING_BALANCE_BTC: Balance = Balance {
    total: dec!(0),
    free: dec!(0),
};
const STARTING_BALANCE_ETH: Balance = Balance {
    total: dec!(0),
    free: dec!(0),
};
const STARTING_BALANCE_SOL: Balance = Balance {
    total: dec!(1),
    free: dec!(1),
};

/// Engine state used for running the custom strategy
type EngineState =
    state::EngineState<DefaultMarketData, BuyAndHoldStrategyState, DefaultRiskManagerState>;

/// The strategy buys remaining quantity required so that our holdings are at
/// least equal to the desired holdings.
struct BuyAndHoldStrategy {
    /// Strategy Id
    id: StrategyId,
    /// Vector of instruments with the desired balances to hold in base asset of
    /// that instrument.
    desired_balances: Vec<(InstrumentIndex, Decimal)>,
}

/// Strategy state that can be mutated
#[derive(Debug, Default, Clone)]
struct BuyAndHoldStrategyState {
    /// Trades that were executed as part of this strategy
    trades: Vec<Trade<QuoteAsset, InstrumentIndex>>,
}

impl Processor<&AccountEvent> for BuyAndHoldStrategyState {
    type Audit = ();

    fn process(&mut self, event: &AccountEvent) -> Self::Audit {
        if let AccountEventKind::Trade(trade) = &event.kind {
            self.trades.push(trade.clone());
        }
    }
}

impl Processor<&MarketEvent> for BuyAndHoldStrategyState {
    type Audit = ();

    fn process(&mut self, _event: &MarketEvent) -> Self::Audit {
        // Update strategy state when we receive a specific MarketEvent
    }
}

impl AlgoStrategy for BuyAndHoldStrategy {
    type State = EngineState;

    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestCancel>>,
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestOpen>>,
    ) {
        let mut open_requests = vec![];

        for (instrument_index, desired_base) in &self.desired_balances {
            let instrument = state.instruments.instrument_index(instrument_index);

            // Balance of the base asset
            let Some(balance) = state
                .assets
                .asset_index(&instrument.instrument.underlying.base)
                .balance
            else {
                continue;
            };

            // Current orders placed for the instrument
            let current_orders = instrument.orders.orders();

            // The balance change we are expecting to get when the open orders
            // get filled.
            let balance_order_delta = current_orders
                .map(|o| match &o.state {
                    InternalOrderState::OpenInFlight(_) => Decimal::ZERO,
                    InternalOrderState::Open(open) => open.quantity_remaining(),
                    // Decimal::ZERO is not entirely correct. The thing is that
                    // the order can still be filled until it's cancellation is
                    // confirmed by the exchange. The calculated delta would be
                    // wrong until the balances are not updated with that fill.
                    InternalOrderState::CancelInFlight(_) => Decimal::ZERO,
                })
                .sum::<Decimal>();

            // TODO: The calculated quota is not correct whatever we do.
            // Currently the MockExchange is not updating the balances of base
            // asset on the order execution. The `balance.value.total` never
            // changes.
            let remaining_buy_quota = desired_base - balance.value.total - balance_order_delta;
            if remaining_buy_quota <= Decimal::ZERO {
                continue;
            }

            // Current market price. This is needed because of how the MockExchange
            // currently mocks order executions. Ideally we wouldn't need to pass
            // this in the future.
            // TODO: Ugly type annotation needed.
            let Some(price) =
                <DefaultMarketData as MarketDataState<InstrumentIndex>>::price(&instrument.market)
            else {
                warn!("Market price is not yet set");
                return (vec![], vec![]);
            };

            // Buy order we are proposing
            open_requests.push(Order {
                exchange: instrument.instrument.exchange,
                instrument: *instrument_index,
                strategy: self.id.clone(),
                cid: ClientOrderId::default(),
                side: Side::Buy,
                state: RequestOpen {
                    kind: OrderKind::Market,
                    time_in_force: TimeInForce::ImmediateOrCancel,
                    price,
                    quantity: remaining_buy_quota,
                },
            });
        }

        (vec![], open_requests)
    }
}

impl ClosePositionsStrategy for BuyAndHoldStrategy {
    type State = EngineState;

    fn close_positions_requests<'a>(
        &'a self,
        _state: &'a Self::State,
        _filter: &'a InstrumentFilter,
    ) -> (
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestCancel>> + 'a,
        impl IntoIterator<Item = Order<ExchangeIndex, InstrumentIndex, RequestOpen>> + 'a,
    )
    where
        ExchangeIndex: 'a,
        AssetIndex: 'a,
        InstrumentIndex: 'a,
    {
        // Here we can specify how should the positions be closed for our
        // specific strategy.
        (std::iter::empty(), std::iter::empty())
    }
}

impl<Clock, State, ExecutionTxs, Risk> OnDisconnectStrategy<Clock, State, ExecutionTxs, Risk>
    for BuyAndHoldStrategy
{
    type OnDisconnect = ();

    fn on_disconnect(
        _engine: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
        _exchange: ExchangeId,
    ) -> Self::OnDisconnect {
        // What to do when we are disconnected from the _exchange?
    }
}

impl<Clock, State, ExecutionTxs, Risk> OnTradingDisabled<Clock, State, ExecutionTxs, Risk>
    for BuyAndHoldStrategy
{
    type OnTradingDisabled = ();

    fn on_trading_disabled(
        _engine: &mut Engine<Clock, State, ExecutionTxs, Self, Risk>,
    ) -> Self::OnTradingDisabled {
        // What to do when we receive the command to disable trading?
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise Tracing
    init_logging();

    // Initialise Channels
    let (feed_tx, mut feed_rx) = mpsc_unbounded();
    let (audit_tx, audit_rx) = mpsc_unbounded();

    // Construct IndexedInstruments
    let instruments = indexed_instruments();

    // Initialise MarketData Stream & forward to Engine feed
    let market_stream = init_indexed_multi_exchange_market_stream(
        &instruments,
        &[SubKind::PublicTrades, SubKind::OrderBooksL1],
    )
    .await?;
    tokio::spawn(market_stream.forward_to(feed_tx.clone()));

    // Construct Engine clock
    let clock = LiveClock;

    // Construct EngineState from IndexedInstruments and hard-coded exchange asset Balances
    let state = EngineState::builder(&instruments)
        .time_engine_start(clock.time())
        // Note: you may want to start to engine with TradingState::Disabled and turn on later
        .trading_state(TradingState::Enabled)
        .strategy(BuyAndHoldStrategyState::default())
        .balances([
            (EXCHANGE, "usdt", STARTING_BALANCE_USDT),
            (EXCHANGE, "btc", STARTING_BALANCE_BTC),
            (EXCHANGE, "eth", STARTING_BALANCE_ETH),
            (EXCHANGE, "sol", STARTING_BALANCE_SOL),
        ])
        // Note: can add other initial data via this builder (eg/ exchange asset balances)
        .build();

    // Generate initial AccountSnapshot from EngineState for BinanceSpot MockExchange
    // Note: for live-trading this would be automatically fetched via the AccountStream init
    let mut initial_account = FnvHashMap::from(&state);
    assert_eq!(initial_account.len(), 1);

    // Initialise ExecutionManager & forward Account Streams to Engine feed
    let (execution_txs, account_stream) = ExecutionBuilder::new(&instruments)
        .add_mock(MockExecutionConfig::new(
            EXCHANGE,
            initial_account.remove(&EXCHANGE).unwrap(),
            MOCK_EXCHANGE_ROUND_TRIP_LATENCY_MS,
            MOCK_EXCHANGE_FEES_PERCENT,
        ))?
        .init()
        .await?;
    tokio::spawn(account_stream.forward_to(feed_tx.clone()));

    let btc_instrument = instruments
        .find_instrument_index(EXCHANGE, &"binance_spot_btc_usdt".into())
        .unwrap();
    let eth_instrument = instruments
        .find_instrument_index(EXCHANGE, &"binance_spot_eth_usdt".into())
        .unwrap();
    let sol_instrument = instruments
        .find_instrument_index(EXCHANGE, &"binance_spot_sol_usdt".into())
        .unwrap();

    // Construct Engine
    let mut engine = Engine::new(
        clock,
        state,
        execution_txs,
        BuyAndHoldStrategy {
            id: StrategyId::new("buy_and_hold"),
            desired_balances: vec![
                (btc_instrument, dec!(0.1)),
                (eth_instrument, dec!(1)),
                (sol_instrument, dec!(10)),
            ],
        },
        DefaultRiskManager::default(),
    );

    // Run synchronous Engine on blocking task
    let engine_task = tokio::task::spawn_blocking(move || {
        let shutdown_audit = run(
            &mut feed_rx,
            &mut engine,
            &mut ChannelTxDroppable::new(audit_tx),
        );
        (engine, shutdown_audit)
    });

    // Run dummy asynchronous AuditStream consumer
    // Note: you probably want to use this Stream to replicate EngineState, or persist events, etc.
    //  --> eg/ see examples/engine_with_replica_engine_state.rs
    let audit_task = tokio::spawn(async move {
        let mut audit_stream = audit_rx.into_stream();
        while let Some(audit) = audit_stream.next().await {
            debug!(?audit, "AuditStream consumed AuditTick");
            if let EngineAudit::Shutdown(_) = audit.event {
                break;
            }
        }
        audit_stream
    });

    // Let the example run for 4 seconds..., then:
    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
    // 1. Disable Strategy order generation (still continues to update EngineState)
    feed_tx.send(TradingState::Disabled)?;
    // 2. Cancel all open orders
    feed_tx.send(Command::CancelOrders(InstrumentFilter::None))?;
    // 3. Send orders to close current positions
    feed_tx.send(Command::ClosePositions(InstrumentFilter::None))?;
    // 4. Stop Engine run loop
    feed_tx.send(EngineEvent::Shutdown)?;

    // Await Engine & AuditStream task graceful shutdown
    // Note: Engine & AuditStream returned, ready for further use
    let (engine, _shutdown_audit) = engine_task.await?;
    let _audit_stream = audit_task.await?;

    // Generate TradingSummary<Daily>
    let trading_summary = engine
        .trading_summary_generator(RISK_FREE_RETURN)
        .generate(Daily);

    // Print TradingSummary<Daily> to terminal (could save in a file, send somewhere, etc.)
    trading_summary.print_summary();

    Ok(())
}

fn indexed_instruments() -> IndexedInstruments {
    IndexedInstruments::builder()
        .add_instrument(Instrument::new(
            EXCHANGE,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            InstrumentKind::Spot,
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.0001), dec!(0.0)),
                InstrumentSpecQuantity::new(
                    OrderQuantityUnits::Quote,
                    dec!(0.00001),
                    dec!(0.00001),
                ),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::new(
            EXCHANGE,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            InstrumentKind::Spot,
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.0001), dec!(0.0001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::new(
            EXCHANGE,
            "binance_spot_sol_usdt",
            "SOLUSDT",
            Underlying::new("sol", "usdt"),
            InstrumentKind::Spot,
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .build()
}

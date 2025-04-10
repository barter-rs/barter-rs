use barter::{
    EngineEvent,
    engine::{
        Engine,
        audit::EngineAudit,
        clock::{EngineClock, LiveClock},
        command::Command,
        run,
        state::{
            EngineState,
            instrument::{
                data::{DefaultInstrumentMarketData, InstrumentDataState},
                filter::InstrumentFilter,
            },
            trading::TradingState,
        },
    },
    execution::builder::ExecutionBuilder,
    logging::init_logging,
    risk::{
        DefaultRiskManagerState, RiskApproved, RiskManager, RiskRefused,
        check::{
            CheckHigherThan, RiskCheck,
            util::{calculate_abs_percent_difference, calculate_quote_notional},
        },
    },
    statistic::time::Daily,
    strategy::{DefaultStrategy, DefaultStrategyState},
};
use barter_data::{
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
        OrderKind,
        request::{OrderRequestCancel, OrderRequestOpen},
    },
};
use barter_instrument::{
    Underlying,
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{
        Instrument,
        kind::InstrumentKind,
        spec::{
            InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            OrderQuantityUnits,
        },
    },
};
use barter_integration::channel::{ChannelTxDroppable, Tx, mpsc_unbounded};
use derive_more::Constructor;
use fnv::FnvHashMap;
use futures::StreamExt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, marker::PhantomData};
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
    total: dec!(0.1),
    free: dec!(0.1),
};
const STARTING_BALANCE_ETH: Balance = Balance {
    total: dec!(1.0),
    free: dec!(1.0),
};
const STARTING_BALANCE_SOL: Balance = Balance {
    total: dec!(10.0),
    free: dec!(10.0),
};

const MAX_MARKET_ORDER_PRICE_PERCENT_FROM_MARKET: CheckHigherThan<Decimal> = CheckHigherThan {
    limit: dec!(0.1), // 10%
};

// All configured Instruments are quoted in usdt
const MAX_USDT_NOTIONAL_PER_ORDER: CheckHigherThan<Decimal> = CheckHigherThan {
    limit: dec!(50.0), // 50 usdt
};

/// Custom risk manager that implements risk checks for orders
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct CustomRiskManager<State> {
    pub max_notional_per_order: CheckHigherThan<Decimal>,
    pub max_market_order_price_percent_from_market: CheckHigherThan<Decimal>,
    phantom: PhantomData<State>,
}

impl<State> Default for CustomRiskManager<State> {
    fn default() -> Self {
        Self {
            max_notional_per_order: MAX_USDT_NOTIONAL_PER_ORDER,
            max_market_order_price_percent_from_market: MAX_MARKET_ORDER_PRICE_PERCENT_FROM_MARKET,
            phantom: PhantomData::default(),
        }
    }
}

impl RiskManager
    for CustomRiskManager<
        EngineState<DefaultInstrumentMarketData, DefaultStrategyState, DefaultRiskManagerState>,
    >
{
    type State =
        EngineState<DefaultInstrumentMarketData, DefaultStrategyState, DefaultRiskManagerState>;

    fn check(
        &self,
        state: &Self::State,
        cancels: impl IntoIterator<Item = OrderRequestCancel>,
        opens: impl IntoIterator<Item = OrderRequestOpen>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<OrderRequestCancel>>,
        impl IntoIterator<Item = RiskApproved<OrderRequestOpen>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestCancel>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestOpen>>,
    ) {
        // Always approve cancel requests (no risk check for cancels)
        let approved_cancels = cancels
            .into_iter()
            .map(RiskApproved::new)
            .collect::<Vec<_>>();

        // Process open order requests with risk checks
        let (approved_opens, refused_opens): (Vec<_>, Vec<_>) = opens
            .into_iter()
            .fold((Vec::new(), Vec::new()), |(mut approved, mut refused), request_open| {
                // Find InstrumentState associated with OrderRequestOpen
                let instrument_state = state
                    .instruments
                    .instrument_index(&request_open.key.instrument);

                if let InstrumentKind::Option(_) = instrument_state.instrument.kind {
                    refused.push(RiskRefused::new(
                        request_open,
                        "RiskManager cannot check Options orders without a strike price"
                    ));
                    return (approved, refused);
                }

                // Calculate notional value in instrument quote currency
                let notional = calculate_quote_notional(
                    request_open.state.quantity,
                    request_open.state.price,
                    instrument_state.instrument.kind.contract_size(),
                ).expect("notional calculation overflowed");

                // Filter orders with a notional higher than current limits
                if let Err(error) = self.max_notional_per_order.check(&notional) {
                    warn!(
                        instrument = %instrument_state.instrument.name_internal,
                        ?request_open,
                        ?error,
                        "RiskManager filtered order: max_notional_per_instrument failed"
                    );
                    refused.push(RiskRefused::new(
                        request_open,
                        "RiskManager max_notional_per_instrument failed"
                    ));
                    return (approved, refused);
                }

                // Only need to make additional checks if OrderKind::Market, so can approve otherwise
                if OrderKind::Market != request_open.state.kind {
                    approved.push(RiskApproved::new(request_open));
                    return (approved, refused);
                }

                // Check there is an instrument market data price available
                let Some(market_price) = instrument_state.data.price() else {
                    warn!(
                        instrument = %instrument_state.instrument.name_internal,
                        ?request_open,
                        market_data = ?instrument_state.data,
                        "RiskManager filtered order: max_market_order_price_percent_from_market failed: no available instrument market price"
                    );
                    refused.push(RiskRefused::new(
                        request_open,
                        "RiskManager max_market_order_price_percent_from_market failed"
                    ));
                    return (approved, refused);
                };

                // Calculate percentage difference from the latest market price
                let price_diff_pct = calculate_abs_percent_difference(
                    request_open.state.price,
                    market_price,
                ).expect("price abs percent difference calculation overflowed");

                // Filter orders with price_diff_pct deviation from the latest market data price
                if let Err(error) = self.max_market_order_price_percent_from_market.check(&price_diff_pct) {
                    warn!(
                        instrument = %instrument_state.instrument.name_internal,
                        ?request_open,
                        ?error,
                        "RiskManager filtered order: max_market_order_price_percent_from_market failed"
                    );
                    refused.push(RiskRefused::new(
                        request_open,
                        "RiskManager max_market_order_price_percent_from_market failed",
                    ));
                    return (approved, refused);
                }

                // All checks passed, approve order
                approved.push(RiskApproved::new(request_open));
                (approved, refused)
            });

        (
            approved_cancels,
            approved_opens,
            std::iter::empty(),
            refused_opens,
        )
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

    // Construct EngineState with our custom risk manager state
    let state = EngineState::<
        DefaultInstrumentMarketData,
        DefaultStrategyState,
        DefaultRiskManagerState,
    >::builder(&instruments)
    .time_engine_start(clock.time())
    .trading_state(TradingState::Enabled)
    .balances([
        (EXCHANGE, "usdt", STARTING_BALANCE_USDT),
        (EXCHANGE, "btc", STARTING_BALANCE_BTC),
        (EXCHANGE, "eth", STARTING_BALANCE_ETH),
        (EXCHANGE, "sol", STARTING_BALANCE_SOL),
    ])
    .build();

    // Generate initial AccountSnapshot from EngineState for BinanceSpot MockExchange
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

    // Construct Engine with our CustomRiskManager
    let mut engine = Engine::new(
        clock,
        state,
        execution_txs,
        DefaultStrategy::default(),
        CustomRiskManager::default(),
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

    // Run asynchronous AuditStream consumer to monitor risk decisions
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

    // Let the example run for 5 seconds..., then:
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // 1. Disable Strategy order generation
    feed_tx.send(TradingState::Disabled)?;
    // 2. Cancel all open orders
    feed_tx.send(Command::CancelOrders(InstrumentFilter::None))?;
    // 3. Close current positions
    feed_tx.send(Command::ClosePositions(InstrumentFilter::None))?;
    // 4. Stop Engine run loop
    feed_tx.send(EngineEvent::Shutdown)?;

    // Await Engine & AuditStream task graceful shutdown
    let (engine, _shutdown_audit) = engine_task.await?;
    let _audit_stream = audit_task.await?;

    // Generate TradingSummary<Daily>
    let trading_summary = engine
        .trading_summary_generator(RISK_FREE_RETURN)
        .generate(Daily);

    // Print TradingSummary<Daily> to terminal
    trading_summary.print_summary();

    Ok(())
}

fn indexed_instruments() -> IndexedInstruments {
    IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            EXCHANGE,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
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
        .add_instrument(Instrument::spot(
            EXCHANGE,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.0001), dec!(0.0001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::spot(
            EXCHANGE,
            "binance_spot_sol_usdt",
            "SOLUSDT",
            Underlying::new("sol", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .build()
}

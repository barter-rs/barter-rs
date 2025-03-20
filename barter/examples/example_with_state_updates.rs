use barter::{
    EngineEvent,
    engine::{
        Engine,
        audit::EngineAudit,
        clock::{EngineClock, HistoricalClock},
        command::Command,
        run,
        state::{
            EngineState,
            instrument::{filter::InstrumentFilter},
            trading::TradingState,
        },
    },
    execution::builder::ExecutionBuilder,
    logging::init_logging,
    risk::{DefaultRiskManager, DefaultRiskManagerState},
    statistic::time::Daily,
    strategy::{DefaultStrategy, DefaultStrategyState},
};
use barter_data::{
    event::DataKind,
    streams::{
        consumer::{MarketStreamEvent, MarketStreamResult},
        reconnect::{Event, stream::ReconnectingStream},
    },
};
use barter_execution::{balance::Balance, client::mock::MockExecutionConfig};
use barter_instrument::{
    Underlying,
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{
        Instrument, InstrumentIndex,
        spec::{
            InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            OrderQuantityUnits,
        },
    },
};
use barter_integration::{channel::{mpsc_unbounded, ChannelTxDroppable, Tx}, collection::one_or_many::OneOrMany};
use derive_more::Constructor;
use fnv::FnvHashMap;
use futures::{Stream, StreamExt, stream};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{debug, info, warn};

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

const FILE_PATH_HISTORIC_TRADES_AND_L1S: &str =
    "barter/examples/data/binance_spot_market_data_with_disconnect_events.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise Tracing
    init_logging();

    // Initialise Channels
    let (feed_tx, mut feed_rx) = mpsc_unbounded();
    let (audit_tx, audit_rx) = mpsc_unbounded();

    // Construct IndexedInstruments
    let instruments = indexed_instruments();

    // Initialise HistoricalClock & MarketStream
    let (clock, market_stream) =
        init_historic_clock_and_market_data_stream(FILE_PATH_HISTORIC_TRADES_AND_L1S);

    // Forward market data events to Engine feed
    tokio::spawn(market_stream.forward_to(feed_tx.clone()));

    // Construct EngineState from IndexedInstruments and hard-coded exchange asset Balances
    let state = EngineState::<
        InstrumentMarketData,
        DefaultStrategyState,
        DefaultRiskManagerState,
    >::builder(&instruments)
    .time_engine_start(clock.time())
    // Note: you may want to start to engine with TradingState::Disabled and turn on later
    .trading_state(TradingState::Enabled)
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

    // Construct Engine
    let mut engine = Engine::new(
        clock,
        state,
        execution_txs,
        DefaultStrategy::default(),
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
            match &audit.event {
                EngineAudit::Process(process_audit) => {
                    // Handle instrument events here
                    match process_audit {
                        barter::engine::audit::ProcessAudit::Process(_) => {}
                        barter::engine::audit::ProcessAudit::ProcessWithOutput(_, one_or_many) => {
                            match one_or_many {
                                OneOrMany::One(engine_output) => {
                                    println!("Engine output: {:?}", engine_output);
                                }
                                OneOrMany::Many(items) => {
                                    println!("Engine outputs: {:?}", items);
                                }
                            }
                        }
                    }
                }
                EngineAudit::Shutdown(_) => break,
                _ => {}
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

// Note that there are far more intelligent ways of streaming historical market data, this is
// just for demonstration purposes.
//
// For example:
// - Stream from database
// - Stream from file
fn init_historic_clock_and_market_data_stream(
    file_path: &str,
) -> (
    HistoricalClock,
    impl Stream<Item = MarketStreamEvent<InstrumentIndex, DataKind>> + use<>,
) {
    let data = std::fs::read_to_string(file_path).unwrap();
    let events =
        serde_json::from_str::<Vec<MarketStreamResult<InstrumentIndex, DataKind>>>(&data).unwrap();

    let time_exchange_first = events
        .iter()
        .find_map(|result| match result {
            MarketStreamResult::Item(Ok(event)) => Some(event.time_exchange),
            _ => None,
        })
        .unwrap();

    let clock = HistoricalClock::new(time_exchange_first);

    let stream = stream::iter(events)
        .with_error_handler(|error| warn!(?error, "MarketStream generated error"))
        .inspect(|event| match event {
            Event::Reconnecting(exchange) => {
                info!(%exchange, "sending historical disconnection to Engine")
            }
            Event::Item(event) => {
                info!(
                    exchange = %event.exchange,
                    instrument = %event.instrument,
                    kind = event.kind.kind_name(),
                    "sending historical event to Engine"
                )
            }
        });

    (clock, stream)
}


/// Provide a custom instrument data state
/// 
/// 

use barter::{
    engine::{state::instrument::data::InstrumentDataState, Processor, WithAudit},
    Timed,
};
use barter_data::{
    event::{ MarketEvent},
    subscription::book::OrderBookL1,
};
use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize, Constructor,
)]
pub struct InstrumentMarketData {
    pub l1: OrderBookL1,
    pub last_traded_price: Option<Timed<Decimal>>,
    //some stateful value... could be a technical indicator on the instrument.
    pub random_stateful_number: i32,
}


#[derive(Debug, Clone)]
pub struct InstrumentAuditTick {
    //this is an optional field and only present when the value has changed from processing the latest event.
    pub random_stateful_number: Option<i32>,
}

// Implement WithAudit for DefaultInstrumentMarketData
impl WithAudit for InstrumentMarketData {
    type Audit = Option<InstrumentAuditTick>;
}

impl InstrumentDataState for InstrumentMarketData {
    type MarketEventKind = DataKind;

    fn price(&self) -> Option<Decimal> {
        self.l1
            .volume_weighed_mid_price()
            .or(self.last_traded_price.as_ref().map(|timed| timed.value))
    }
}

impl<InstrumentKey> Processor<&MarketEvent<InstrumentKey, DataKind>>
    for InstrumentMarketData
{
    fn process(&mut self, event: &MarketEvent<InstrumentKey, DataKind>) -> Self::Audit {
        match &event.kind {
            DataKind::Trade(trade) => {
                if self
                    .last_traded_price
                    .as_ref()
                    .is_none_or(|price| price.time < event.time_exchange)
                {
                    if let Some(price) = Decimal::from_f64(trade.price) {
                        self.last_traded_price
                            .replace(Timed::new(price, event.time_exchange));
                    }
                }
            }
            DataKind::OrderBookL1(l1) => {
                if self.l1.last_update_time < event.time_exchange {
                    self.l1 = l1.clone()
                }
            }
            _ => {}
        }

        //we process the market event and perhaps our indicator has changed.
        self.random_stateful_number = self.random_stateful_number + 1;

        return Some(InstrumentAuditTick {
            random_stateful_number: Some(self.random_stateful_number),
        });
    }
}

pub trait OurIndicatorTrait {
    fn random_stateful_number_current(&self) -> i32;
}


impl OurIndicatorTrait for InstrumentMarketData {
    fn random_stateful_number_current(&self) -> i32 {
        self.random_stateful_number
    }

}
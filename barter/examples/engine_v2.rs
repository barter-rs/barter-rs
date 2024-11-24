use barter::v2::{
    engine::{
        command::Command,
        run,
        state::{
            instrument::{manager::InstrumentFilter, market_data::DefaultMarketData},
            trading::TradingState,
            EngineState,
        },
        Engine,
    },
    error::BarterError,
    execution::{
        builder::ExecutionBuilder,
        manager::client::{MockExecution, MockExecutionConfig},
    },
    instrument::IndexedInstruments,
    risk::{DefaultRiskManager, DefaultRiskManagerState},
    strategy::{DefaultStrategy, DefaultStrategyState},
    EngineEvent,
};
use barter_data::{
    event::DataKind,
    streams::{
        builder::dynamic::DynamicStreams,
        consumer::{MarketStreamEvent, MarketStreamResult},
        reconnect::stream::ReconnectingStream,
    },
    subscription::{SubKind, Subscription},
};
use barter_instrument::{
    asset::Asset,
    exchange::ExchangeId,
    instrument::{
        kind::InstrumentKind,
        market_data::MarketDataInstrument,
        spec::{
            InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            OrderQuantityUnits,
        },
        Instrument, InstrumentIndex,
    },
    Underlying,
};
use barter_integration::channel::{mpsc_unbounded, ChannelTxDroppable, Tx};
use chrono::Utc;
use futures::Stream;
use rust_decimal_macros::dec;
use tracing::{info, warn};

const EXECUTION_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise Tracing
    init_logging();

    // Initialise Channels
    let (feed_tx, mut feed_rx) = mpsc_unbounded();
    let (audit_tx, audit_rx) = mpsc_unbounded();

    // Construct IndexedInstruments
    let instruments = IndexedInstruments::new(unindexed_instruments())?;

    // Initialise MarketData Stream & forward to Engine feed
    let stream = init_market_data_stream(&instruments).await?;
    tokio::spawn(stream.forward_to(feed_tx.clone()));

    // Initialise ExecutionManager & forward Account Streams to Engine feed
    let (execution_txs, account_stream) = ExecutionBuilder::new(&instruments)
        .add::<MockExecution>(MockExecutionConfig, EXECUTION_REQUEST_TIMEOUT)?
        .init()
        .await?;
    tokio::spawn(account_stream.forward_to(feed_tx.clone()));

    // Construct EngineState
    let state = EngineState {
        trading: TradingState::Disabled,
        connectivity: instruments.connectivity_states(),
        assets: instruments.asset_states(),
        instruments: instruments.instrument_states::<DefaultMarketData>(),
        strategy: DefaultStrategyState,
        risk: DefaultRiskManagerState,
    };

    let mut engine = Engine::new(
        || Utc::now(),
        state,
        execution_txs,
        DefaultStrategy::default(),
        DefaultRiskManager::default(),
    );

    // Run synchronous Engine on blocking task
    let engine_task = tokio::task::spawn_blocking(move || {
        run(
            &mut feed_rx,
            &mut engine,
            &mut ChannelTxDroppable::new(audit_tx),
        )
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

    // Await Engine task graceful shutdown
    let shutdown_audit = engine_task.await?;
    info!(?shutdown_audit, "Engine shutdown");
    Ok(())
}

fn init_logging() {
    tracing_subscriber::fmt()
        // Filter messages based on the INFO
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        // Disable colours on release builds
        .with_ansi(cfg!(debug_assertions))
        // Enable Json formatting
        .json()
        // Install this Tracing subscriber as global default
        .init()
}

fn unindexed_instruments() -> Vec<Instrument<ExchangeId, Asset>> {
    vec![
        Instrument::new(
            ExchangeId::BinanceSpot,
            "btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            InstrumentKind::Spot,
            InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.0001), dec!(0.0)),
                InstrumentSpecQuantity::new(
                    OrderQuantityUnits::Quote,
                    dec!(0.00001),
                    dec!(0.00001),
                ),
                InstrumentSpecNotional::new(dec!(5.0)),
            ),
        ),
        Instrument::new(
            ExchangeId::BinanceSpot,
            "eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            InstrumentKind::Spot,
            InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.0001), dec!(0.0001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            ),
        ),
        Instrument::new(
            ExchangeId::BinanceSpot,
            "sol_usdt",
            "SOLUSDT",
            Underlying::new("sol", "usdt"),
            InstrumentKind::Spot,
            InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            ),
        ),
    ]
}

async fn init_market_data_stream(
    instruments: &IndexedInstruments,
) -> Result<impl Stream<Item = MarketStreamEvent<InstrumentIndex, DataKind>>, BarterError> {
    // Construct Indexed MarketData Subscriptions
    let data_subscriptions = instruments
        .market_data_subscriptions(unindexed_market_data_subscriptions(&unindexed_instruments()))?;

    // Initialise MarketData Stream
    let stream = DynamicStreams::init(data_subscriptions)
        .await?
        .select_all::<MarketStreamResult<InstrumentIndex, DataKind>>()
        .with_error_handler(|error| warn!(?error, "MarketStream generated error"));

    Ok(stream)
}

fn unindexed_market_data_subscriptions(
    instruments: &[Instrument<ExchangeId, Asset>],
) -> impl IntoIterator<Item = Vec<Subscription>> {
    let (trades, l1s) = instruments
        .iter()
        .map(|instrument| {
            (
                Subscription::new(
                    instrument.exchange,
                    MarketDataInstrument::from(instrument),
                    SubKind::PublicTrades,
                ),
                Subscription::new(
                    instrument.exchange,
                    MarketDataInstrument::from(instrument),
                    SubKind::OrderBooksL1,
                ),
            )
        })
        .unzip();

    [trades, l1s]
}

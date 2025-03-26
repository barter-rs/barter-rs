use barter::{
    engine::{
        clock::HistoricalClock,
        state::{instrument::data::DefaultInstrumentMarketData, trading::TradingState},
    },
    logging::init_logging,
    risk::{DefaultRiskManager, DefaultRiskManagerState},
    strategy::{DefaultStrategy, DefaultStrategyState},
    system::{
        AuditMode, EngineFeedMode, ExecutionConfig,
        builder::{SystemBuilder, SystemConfig},
    },
};
use barter_execution::{AccountSnapshot, client::mock::MockExecutionConfig};
use barter_instrument::{
    Underlying,
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{
        Instrument,
        spec::{
            InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            OrderQuantityUnits,
        },
    },
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::{sync::Arc, time::Duration};
use futures::{stream, Stream};
use tracing::{info, warn};
use barter_data::event::DataKind;
use barter_data::streams::consumer::{MarketStreamEvent, MarketStreamResult};
use barter_data::streams::reconnect::Event;
use barter_data::streams::reconnect::stream::ReconnectingStream;
use barter_instrument::instrument::InstrumentIndex;
use futures::StreamExt;

const EXCHANGE: ExchangeId = ExchangeId::BinanceSpot;
const FILE_PATH_HISTORIC_TRADES_AND_L1S: &str =
    "barter/examples/data/binance_spot_market_data_with_disconnect_events.json";
const MOCK_EXCHANGE_ROUND_TRIP_LATENCY_MS: u64 = 100;
const MOCK_EXCHANGE_FEES_PERCENT: Decimal = dec!(0.05);
const FILE_PATH_MARKET_DATA: &str = "../../content/backtest/market_data/indexed";
const RISK_FREE_RETURN: Decimal = dec!(0.05);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise Tracing
    init_logging();

    // Construct IndexedInstruments
    let instruments = Arc::new(indexed_instruments());

    // Initialise HistoricalClock & MarketStream
    let (clock, market_stream) =
        init_historic_clock_and_market_data_stream(FILE_PATH_HISTORIC_TRADES_AND_L1S);

    let config = SystemConfig {
        instruments,
        executions: vec![ExecutionConfig::Mock(MockExecutionConfig {
            mocked_exchange: EXCHANGE,
            initial_state: AccountSnapshot {
                exchange: EXCHANGE,
                balances: vec![],
                instruments: vec![],
            },
            latency_ms: MOCK_EXCHANGE_ROUND_TRIP_LATENCY_MS,
            fees_percent: MOCK_EXCHANGE_FEES_PERCENT,
        })],
        clock,
        strategy: DefaultStrategy::default(),
        risk: DefaultRiskManager::default(),
        market_stream,
    };

    let mut system = SystemBuilder::new(config)
        .runtime(tokio::runtime::Handle::current())
        .engine_feed_mode(EngineFeedMode::Async)
        .audit_mode(AuditMode::Enabled)
        .trading_state(TradingState::Enabled)
        .build::<DefaultInstrumentMarketData, DefaultStrategyState, DefaultRiskManagerState>()
        .await?;

    let (
        engine,
        shutdown_audit
    ) = system.handles.engine.await?;

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

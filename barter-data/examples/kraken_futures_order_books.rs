use barter_data::{
    exchange::kraken::KrakenFuturesUsd,
    streams::{reconnect::stream::ReconnectingStream, Streams},
    subscription::book::{OrderBooksL1, OrderBooksL2},
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use futures_util::StreamExt;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    init_logging();

    info!("Initialising L1 Order Book Streams...");

    // Initialise OrderBooksL1 Streams for KrakenFuturesUsd
    // L1 = Best bid and ask (top of book)
    let l1_streams = Streams::<OrderBooksL1>::builder()
        .subscribe([
            (
                KrakenFuturesUsd::default(),
                "btc",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                OrderBooksL1,
            ),
            (
                KrakenFuturesUsd::default(),
                "eth",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                OrderBooksL1,
            ),
        ])
        .init()
        .await
        .unwrap();

    info!("L1 Streams Initialised.");

    info!("Initialising L2 Order Book Streams...");

    // Initialise OrderBooksL2 Streams for KrakenFuturesUsd
    // L2 = Full order book depth
    let l2_streams = Streams::<OrderBooksL2>::builder()
        .subscribe([
            (
                KrakenFuturesUsd::default(),
                "xrp",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                OrderBooksL2,
            ),
            (
                KrakenFuturesUsd::default(),
                "sol",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                OrderBooksL2,
            ),
        ])
        .init()
        .await
        .unwrap();

    info!("L2 Streams Initialised. Joining all streams...");

    // Handle L1 streams
    let l1_handle = tokio::spawn(async move {
        let mut l1_stream = l1_streams
            .select_all()
            .with_error_handler(|error| warn!(?error, "L1 MarketStream generated error"));

        while let Some(event) = l1_stream.next().await {
            // event is MarketEvent<InstrumentKey, OrderBookEvent>
            info!("L1: {event:?}");
        }
    });

    // Handle L2 streams
    let l2_handle = tokio::spawn(async move {
        let mut l2_stream = l2_streams
            .select_all()
            .with_error_handler(|error| warn!(?error, "L2 MarketStream generated error"));

        while let Some(event) = l2_stream.next().await {
            // event is MarketEvent<InstrumentKey, OrderBookEvent>
            info!("L2: {event:?}");
        }
    });

    // Wait for both tasks
    let _ = tokio::join!(l1_handle, l2_handle);
}

// Initialise an INFO `Subscriber` for `Tracing` Json logs and install it as the global default.
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

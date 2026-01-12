use barter_data::{
    exchange::kraken::KrakenFuturesUsd,
    streams::{reconnect::stream::ReconnectingStream, Streams},
    subscription::trade::PublicTrades,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use futures_util::StreamExt;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    init_logging();

    info!("Initialising Streams...");

    // Initialise PublicTrades Streams for KrakenFuturesUsd
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let streams = Streams::<PublicTrades>::builder()
        // Separate WebSocket connection for BTC perpetual since it's high volume
        .subscribe([(
            KrakenFuturesUsd::default(),
            "btc",
            "usd",
            MarketDataInstrumentKind::Perpetual,
            PublicTrades,
        )])
        // Separate WebSocket connection for ETH perpetual
        .subscribe([(
            KrakenFuturesUsd::default(),
            "eth",
            "usd",
            MarketDataInstrumentKind::Perpetual,
            PublicTrades,
        )])
        // Lower volume instruments can share a WebSocket connection
        .subscribe([
            (
                KrakenFuturesUsd::default(),
                "xrp",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                PublicTrades,
            ),
            (
                KrakenFuturesUsd::default(),
                "sol",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                PublicTrades,
            ),
        ])
        .init()
        .await
        .unwrap();

    info!("Streams Initialised. Joining...");

    // Select and merge every exchange Stream
    let mut joined_stream = streams
        .select_all()
        .with_error_handler(|error| warn!(?error, "MarketStream generated error"));

    while let Some(event) = joined_stream.next().await {
        // event is MarketEvent<InstrumentKey, PublicTrade>
        info!("{event:?}");
    }
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

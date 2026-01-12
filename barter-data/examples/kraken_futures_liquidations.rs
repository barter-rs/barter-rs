use barter_data::{
    exchange::kraken::KrakenFuturesUsd,
    streams::{reconnect::stream::ReconnectingStream, Streams},
    subscription::liquidation::Liquidations,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use futures_util::StreamExt;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    init_logging();

    info!("Initialising Streams...");

    // Initialise Liquidations Streams for KrakenFuturesUsd
    // Note: Kraken Futures streams liquidations via the trade feed with trade type "liquidation"
    let streams = Streams::<Liquidations>::builder()
        .subscribe([
            (
                KrakenFuturesUsd::default(),
                "btc",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                Liquidations,
            ),
            (
                KrakenFuturesUsd::default(),
                "eth",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                Liquidations,
            ),
            (
                KrakenFuturesUsd::default(),
                "xrp",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                Liquidations,
            ),
            (
                KrakenFuturesUsd::default(),
                "sol",
                "usd",
                MarketDataInstrumentKind::Perpetual,
                Liquidations,
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
        // event is MarketEvent<InstrumentKey, Liquidation>
        // Note: Liquidations are relatively rare events
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

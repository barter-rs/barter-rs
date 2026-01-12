use barter_data::{
    exchange::kraken::KrakenSpot,
    streams::Streams,
    subscription::book::OrderBooksL2,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use futures_util::StreamExt;
use tracing::info;

#[tokio::main]
async fn main() {
    init_logging();

    info!("Initialising Streams...");

    // Initialise OrderBooksL2 Streams for KrakenSpot
    let streams = Streams::<OrderBooksL2>::builder()
        .subscribe([
            (KrakenSpot::default(), "xbt", "usd", MarketDataInstrumentKind::Spot, OrderBooksL2),
        ])
        .init()
        .await
        .unwrap();

    info!("Streams Initialised. Joining...");

    let mut joined_stream = streams.select_all();

    while let Some(event) = joined_stream.next().await {
         // event is MarketEvent<InstrumentKey, OrderBookEvent>
         info!("Event: {event:?}");
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

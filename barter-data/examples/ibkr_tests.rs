use std::io::Error;

use barter_data::{
  exchange::{ibkr::Ibkr, ExchangeId},
  streams::Streams, subscription::book::OrderBooksL1,
};
use barter_integration::model::instrument::kind::InstrumentKind::Spot;
use tracing::{info, Level};
use tracing_subscriber::{filter, fmt, Layer, prelude::*};

#[tokio::main]
async fn main() {
    // Initialise Tracing log subscriber
    let _ = init_logging();

    // Initialise MarketData Streams for Interactive Brokers only
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let mut streams = Streams::<OrderBooksL1>::builder()
        .subscribe([
            (Ibkr::default(), "aapl", "usd", Spot, OrderBooksL1),
        ])
        .init()
        .await
        .unwrap();

    // Select the ExchangeId::Ibkr stream
    // Notes:
    //  - Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    //  - Use `streams.join()` to join all exchange streams into a single mpsc::UnboundedReceiver!
    let mut ibkr_stream = streams
        .select(ExchangeId::Ibkr)
        .unwrap();

    while let Some(msg) = ibkr_stream.recv().await {
        info!(?msg);
    }
}

// Initialise a `Subscriber` for `Tracing` Json logs and install it as the global default.
fn init_logging() -> Result<(), Error> {
    // stdout log
    let stdout_log = fmt::layer().pretty();

    // // json file log
    // let file = File::create("debug_log.json")?;
    // let debug_log = fmt::layer()
    // .with_writer(Arc::new(file))
    // .json();

    let barter_filter = filter::Targets::new()
        .with_target("barter", Level::DEBUG);

    tracing_subscriber::registry()
        // Filter messages based on their level
        .with(barter_filter)
        .with(stdout_log
            .with_filter(
                tracing_subscriber::filter::EnvFilter::builder()
                    .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
        )
        // .with(debug_log)

        // // Disable colours on release builds
        // .with_ansi(cfg!(debug_assertions))

        // // Enable Json formatting
        // .json()

        // Install this Tracing subscriber as global default
        .init();

    Ok(())
}
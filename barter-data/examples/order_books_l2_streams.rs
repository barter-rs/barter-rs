use barter_data::{
    exchange::binance::spot::BinanceSpot,
    streams::{Streams, reconnect::stream::ReconnectingStream},
    subscription::book::OrderBooksL2,
};
use barter_instrument::{
    exchange::ExchangeId, instrument::market_data::kind::MarketDataInstrumentKind,
};
use futures_util::StreamExt;
use tracing::{info, warn};

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise OrderBooksL2 Streams for BinanceSpot only
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let mut streams = Streams::<OrderBooksL2>::builder()

        // Separate WebSocket connection for BTC_USDT stream since it's very high volume
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
        ])

        // Separate WebSocket connection for ETH_USDT stream since it's very high volume
        .subscribe([
            (BinanceSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
        ])

        // Lower volume Instruments can share a WebSocket connection
        .subscribe([
            (BinanceSpot::default(), "xrp", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
            (BinanceSpot::default(), "sol", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
            (BinanceSpot::default(), "avax", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
            (BinanceSpot::default(), "ltc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
        ])
        .init()
        .await
        .unwrap();

    // Select the ExchangeId::BinanceSpot stream
    // Note: use `Streams.select(ExchangeId)` to interact with individual exchange streams!
    let mut l1_stream = streams
        .select(ExchangeId::BinanceSpot)
        .unwrap()
        .with_error_handler(|error| warn!(?error, "MarketStream generated error"));

    while let Some(event) = l1_stream.next().await {
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

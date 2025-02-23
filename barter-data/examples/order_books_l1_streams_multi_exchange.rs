use barter_data::{
    exchange::binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
    streams::{Streams, reconnect::stream::ReconnectingStream},
    subscription::book::OrderBooksL1,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use futures::StreamExt;
use tracing::{info, warn};

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise OrderBooksL1 Streams for various exchanges
    // '--> each call to StreamBuilder::subscribe() initialises a separate WebSocket connection
    let streams = Streams::<OrderBooksL1>::builder()
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL1),
            (BinanceSpot::default(), "eth", "usd", MarketDataInstrumentKind::Spot, OrderBooksL1),
        ])
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, OrderBooksL1),
            (BinanceFuturesUsd::default(), "eth", "usd", MarketDataInstrumentKind::Perpetual, OrderBooksL1),
        ])
        .init()
        .await
        .unwrap();

    // Select and merge every exchange Stream using futures_util::stream::select_all
    // Note: use `Streams.select(ExchangeId)` to interact with individual exchange streams!
    let mut joined_stream = streams
        .select_all()
        .with_error_handler(|error| warn!(?error, "MarketStream generated error"));

    while let Some(event) = joined_stream.next().await {
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

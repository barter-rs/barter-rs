use barter_data::{
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        mexc::MexcSpot,
    },
    streams::{Streams, reconnect::stream::ReconnectingStream},
    subscription::book::OrderBooksL2,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use futures_util::StreamExt;
use tracing::{info, warn};

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise OrderBooksL2 Streams for multiple exchanges
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let streams = Streams::<OrderBooksL2>::builder()

        // BinanceSpot BTC_USDT L2 orderbook
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
        ])

        // BinanceFuturesUsd BTC_USDT perpetual L2 orderbook
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, OrderBooksL2),
        ])

        // MexcSpot ETH_USDT L2 orderbook (uses protobuf transport)
        .subscribe([
            (MexcSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
        ])

        // MexcSpot BTC_USDT L2 orderbook
        .subscribe([
            (MexcSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
        ])

        .init()
        .await
        .unwrap();

    // Select and merge all exchange streams using futures_util::stream::select_all
    // Note: use `Streams.select(ExchangeId)` to interact with individual exchange streams!
    let mut merged_stream = streams
        .select_all()
        .with_error_handler(|error| warn!(?error, "MarketStream generated error"));

    while let Some(event) = merged_stream.next().await {
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

use barter_data::{
    event::DataKind,
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        okx::Okx,
    },
    streams::{Streams, consumer::MarketStreamResult, reconnect::stream::ReconnectingStream},
    subscription::{
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
};
use barter_instrument::instrument::market_data::{
    MarketDataInstrument, kind::MarketDataInstrumentKind,
};
use tokio_stream::StreamExt;
use tracing::{info, warn};

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Notes:
    // - MarketStreamResult<_, DataKind> could use a custom enumeration if more flexibility is required.
    // - Each call to StreamBuilder::subscribe() creates a separate WebSocket connection for the
    //   Subscriptions passed.

    // Initialise MarketEvent<DataKind> Streams for various exchanges
    let streams: Streams<MarketStreamResult<MarketDataInstrument, DataKind>> = Streams::builder_multi()

        // Add PublicTrades Streams for various exchanges
        .add(Streams::<PublicTrades>::builder()
            .subscribe([
                (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            ])
            .subscribe([
                (Okx, "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
                (Okx, "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            ])
        )

        // Add OrderBooksL1 Stream for various exchanges
        .add(Streams::<OrderBooksL1>::builder()
            .subscribe([
                (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL1),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, OrderBooksL1),
            ])
        )

        // Add OrderBooksL2 Stream for various exchanges
        .add(Streams::<OrderBooksL2>::builder()
            .subscribe([
                (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL2),
            ])
            .subscribe([
                (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, OrderBooksL2),
            ])
        )
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

use barter_data::{
    event::DataKind,
    exchange::mexc::Mexc,
    streams::{Streams, consumer::MarketStreamResult, reconnect::stream::ReconnectingStream},
    subscription::{book::OrderBooksL1, trade::PublicTrades},
};
use barter_instrument::instrument::market_data::{
    MarketDataInstrument, kind::MarketDataInstrumentKind,
};
use futures_util::StreamExt;
use tracing::{info, warn};

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    init_logging();

    // Initialise PublicTrades & OrderBooksL1 Streams for Mexc
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let streams: Streams<MarketStreamResult<MarketDataInstrument, DataKind>> =
        Streams::builder_multi()
        .add(
            Streams::<PublicTrades>::builder().subscribe([
                (Mexc, "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
                (Mexc, "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            ]),
        )
        .add(
            Streams::<OrderBooksL1>::builder().subscribe([
                (Mexc, "eth", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL1),
                (Mexc, "btc", "usdt", MarketDataInstrumentKind::Spot, OrderBooksL1),
            ]),
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

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_ansi(cfg!(debug_assertions))
        .json()
        .init();
}

use barter_data::{
    event::{DataKind, MarketEvent},
    exchange::ExchangeId,
    streams::builder::dynamic::DynamicStreams,
    subscription::SubKind,
};
use barter_integration::model::instrument::{kind::InstrumentKind, Instrument};
use futures::StreamExt;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    use ExchangeId::*;
    use InstrumentKind::*;
    use SubKind::*;

    // Notes:
    // - DynamicStream::init requires an IntoIterator<Item = "subscription batch">.
    // - Each "subscription batch" is an IntoIterator<Item = Subscription>.
    // - Every "subscription batch" will initialise at-least-one WebSocket stream under the hood.
    // - If the "subscription batch" contains more-than-one ExchangeId and/or SubKind, the batch
    //   will be further split under the hood for compile-time reasons.

    // Initialise MarketEvent streams for various ExchangeIds and SubscriptionKinds
    let streams = DynamicStreams::init([
        // Batch notes:
        // Since batch contains 1 ExchangeId and 1 SubscriptionKind, so only 1 (1x1) WebSockets
        // will be spawned for this batch.
        vec![
            (BinanceSpot, "btc", "usdt", Spot, PublicTrades),
            (BinanceSpot, "eth", "usdt", Spot, PublicTrades),
        ],

        // Batch notes:
        // Since batch contains 1 ExchangeId and 3 SubscriptionKinds, 3 (1x3) WebSocket connections
        // will be spawned for this batch (back-end requires to further split).
        vec![
            (BinanceFuturesUsd, "btc", "usdt", Perpetual, PublicTrades),
            (BinanceFuturesUsd, "btc", "usdt", Perpetual, OrderBooksL1),
            (BinanceFuturesUsd, "btc", "usdt", Perpetual, Liquidations),

        ],

        // Batch notes:
        // Since batch contains 2 ExchangeIds and 1 SubscriptionKind, 2 (2x1) WebSocket connections
        // will be spawned for this batch (back-end requires to further split).
        vec![
            (Okx, "btc", "usdt", Spot, PublicTrades),
            (Okx, "btc", "usdt", Perpetual, PublicTrades),
            (Bitmex, "btc", "usdt", Perpetual, PublicTrades),
            (Okx, "eth", "usdt", Spot, PublicTrades),
            (Okx, "eth", "usdt", Perpetual, PublicTrades),
            (Bitmex, "eth", "usdt", Perpetual, PublicTrades),
        ],
    ]).await.unwrap();

    // Select all streams, mapping each SubscriptionKind `MarketEvent<T>` into a unified
    // `Output` (eg/ `MarketEvent<Instrument, DataKind>`), where MarketEvent<T>: Into<Output>
    // Notes on other DynamicStreams methods:
    //  - Use `streams.select_trades(ExchangeId)` to return a stream of trades from a given exchange.
    //  - Use `streams.select_<T>(ExchangeId)` to return a stream of T from a given exchange.
    //  - Use `streams.select_all_trades(ExchangeId)` to return a stream of trades from all exchanges
    let mut merged = streams
        .select_all::<MarketEvent<Instrument, DataKind>>();

    while let Some(event) = merged.next().await {
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

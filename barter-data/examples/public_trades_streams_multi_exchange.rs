use barter_data::{
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        bitmex::Bitmex,
        bybit::{futures::BybitPerpetualsUsd, spot::BybitSpot},
        coinbase::Coinbase,
        gateio::{
            option::GateioOptions,
            perpetual::{GateioPerpetualsBtc, GateioPerpetualsUsd},
            spot::GateioSpot,
        },
        okx::Okx,
    },
    streams::Streams,
    subscription::trade::PublicTrades,
};
use barter_integration::model::instrument::kind::{
    FutureContract, InstrumentKind, OptionContract, OptionExercise, OptionKind,
};
use chrono::{TimeZone, Utc};
use futures::StreamExt;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise PublicTrades Streams for various exchanges
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let streams = Streams::<PublicTrades>::builder()
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
            (BinanceSpot::default(), "eth", "usdt", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
            (BinanceFuturesUsd::default(), "eth", "usdt", InstrumentKind::Perpetual, PublicTrades),
        ])
        .subscribe([
            (Coinbase, "btc", "usd", InstrumentKind::Spot, PublicTrades),
            (Coinbase, "eth", "usd", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (GateioSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (GateioPerpetualsUsd::default(), "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
        ])
        .subscribe([
            (GateioPerpetualsBtc::default(), "btc", "usd", InstrumentKind::Perpetual, PublicTrades),
        ])
        .subscribe([
            (GateioOptions::default(), "btc", "usdt", InstrumentKind::Option(put_contract()), PublicTrades),
        ])
        .subscribe([
            (Okx, "btc", "usdt", InstrumentKind::Spot, PublicTrades),
            (Okx, "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
            (Okx, "btc", "usd", InstrumentKind::Future(future_contract()), PublicTrades),
            (Okx, "btc", "usd", InstrumentKind::Option(call_contract()), PublicTrades),
        ])
        .subscribe([
            (BybitSpot::default(), "btc", "usdt", InstrumentKind::Spot, PublicTrades),
            (BybitSpot::default(), "eth", "usdt", InstrumentKind::Spot, PublicTrades),
        ])
        .subscribe([
            (BybitPerpetualsUsd::default(), "btc", "usdt", InstrumentKind::Perpetual, PublicTrades),
        ])
        .subscribe([
            (Bitmex, "xbt", "usd", InstrumentKind::Perpetual, PublicTrades)
        ])
        .init()
        .await
        .unwrap();

    // Join all exchange PublicTrades streams into a single tokio_stream::StreamMap
    // Notes:
    //  - Use `streams.select(ExchangeId)` to interact with the individual exchange streams!
    //  - Use `streams.join()` to join all exchange streams into a single mpsc::UnboundedReceiver!
    let mut joined_stream = streams.join_map().await;

    while let Some((exchange, trade)) = joined_stream.next().await {
        info!("Exchange: {exchange}, MarketEvent<PublicTrade>: {trade:?}");
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

fn put_contract() -> OptionContract {
    OptionContract {
        kind: OptionKind::Put,
        exercise: OptionExercise::European,
        expiry: Utc.timestamp_millis_opt(1703808000000).unwrap(),
        strike: rust_decimal_macros::dec!(50000),
    }
}

fn future_contract() -> FutureContract {
    FutureContract {
        expiry: Utc.timestamp_millis_opt(1695945600000).unwrap(),
    }
}

fn call_contract() -> OptionContract {
    OptionContract {
        kind: OptionKind::Call,
        exercise: OptionExercise::American,
        expiry: Utc.timestamp_millis_opt(1703808000000).unwrap(),
        strike: rust_decimal_macros::dec!(35000),
    }
}

use barter_data::{
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
        bitmex::Bitmex,
        bybit::{futures::BybitPerpetualsUsd, spot::BybitSpot},
        gateio::{
            option::GateioOptions,
            perpetual::{GateioPerpetualsBtc, GateioPerpetualsUsd},
            spot::GateioSpot,
        },
        okx::Okx,
    },
    streams::{Streams, reconnect::stream::ReconnectingStream},
    subscription::trade::PublicTrades,
};
use barter_instrument::instrument::{
    kind::option::{OptionExercise, OptionKind},
    market_data::kind::{
        MarketDataFutureContract, MarketDataInstrumentKind, MarketDataOptionContract,
    },
};
use chrono::{TimeZone, Utc};
use futures::StreamExt;
use tracing::{info, warn};

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise PublicTrades Streams for various exchanges
    // '--> each call to StreamBuilder::subscribe() creates a separate WebSocket connection
    let streams = Streams::<PublicTrades>::builder()
        .subscribe([
            (BinanceSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (BinanceSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
        ])

        .subscribe([
            (BinanceFuturesUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            (BinanceFuturesUsd::default(), "eth", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
        ])

        .subscribe([
            (GateioSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
        ])

        .subscribe([
            (GateioPerpetualsUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
        ])

        .subscribe([
            (GateioPerpetualsBtc::default(), "btc", "usd", MarketDataInstrumentKind::Perpetual, PublicTrades),
        ])

        .subscribe([
            (GateioOptions::default(), "btc", "usdt", MarketDataInstrumentKind::Option(put_contract()), PublicTrades),
        ])

        .subscribe([
            (Okx, "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (Okx, "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
            (Okx, "btc", "usd", MarketDataInstrumentKind::Future(future_contract_expiry()), PublicTrades),
            (Okx, "btc", "usd", MarketDataInstrumentKind::Option(call_contract()), PublicTrades),
        ])

        .subscribe([
            (BybitSpot::default(), "btc", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
            (BybitSpot::default(), "eth", "usdt", MarketDataInstrumentKind::Spot, PublicTrades),
        ])

        .subscribe([
            (BybitPerpetualsUsd::default(), "btc", "usdt", MarketDataInstrumentKind::Perpetual, PublicTrades),
        ])

        .subscribe([
            (Bitmex, "xbt", "usd", MarketDataInstrumentKind::Perpetual, PublicTrades)
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

fn put_contract() -> MarketDataOptionContract {
    let expiry = Utc.timestamp_millis_opt(1758844800000).unwrap();
    if expiry < Utc::now() {
        panic!("Put contract has expired, please configure a non-expired instrument")
    }
    MarketDataOptionContract {
        kind: OptionKind::Put,
        exercise: OptionExercise::European,
        expiry,
        strike: rust_decimal_macros::dec!(70000),
    }
}

fn future_contract_expiry() -> MarketDataFutureContract {
    let expiry = Utc.timestamp_millis_opt(1743120000000).unwrap();
    if expiry < Utc::now() {
        panic!("Future contract has expired, please configure a non-expired instrument")
    }
    MarketDataFutureContract { expiry }
}

fn call_contract() -> MarketDataOptionContract {
    let expiry = Utc.timestamp_millis_opt(1758844800000).unwrap();
    if expiry < Utc::now() {
        panic!("Future contract has expired, please configure a non-expired instrument")
    }

    MarketDataOptionContract {
        kind: OptionKind::Call,
        exercise: OptionExercise::American,
        expiry,
        strike: rust_decimal_macros::dec!(70000),
    }
}

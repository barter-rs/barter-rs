use barter_data::{
    books::{manager::init_multi_order_book_l2_manager, map::OrderBookMap},
    exchange::binance::spot::BinanceSpot,
    subscription::book::OrderBooksL2,
};
use barter_integration::model::instrument::{kind::InstrumentKind, Instrument};
use std::time::Duration;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Initialise OrderBookL2Manager with desired Subscriptions
    let book_manager = init_multi_order_book_l2_manager([
        // Separate WebSocket connection for BTC_USDT stream since it's very high volume
        vec![
            (BinanceSpot::default(), "btc", "usdt", InstrumentKind::Spot, OrderBooksL2)
        ],

        // Separate WebSocket connection for ETH_USDT stream since it's very high volume
        vec![
            (BinanceSpot::default(), "eth", "usdt", InstrumentKind::Spot, OrderBooksL2)
        ],

        // Lower volume Instruments can share a WebSocket connection
        vec![
            (BinanceSpot::default(), "xrp", "usdt", InstrumentKind::Spot, OrderBooksL2),
            (BinanceSpot::default(), "sol", "usdt", InstrumentKind::Spot, OrderBooksL2),
            (BinanceSpot::default(), "avax", "usdt", InstrumentKind::Spot, OrderBooksL2),
            (BinanceSpot::default(), "ltc", "usdt", InstrumentKind::Spot, OrderBooksL2),
        ]
    ]).await.unwrap();

    // Clone OrderBookMap so you can access the locally managed OrderBooks elsewhere in your program
    let books = book_manager.books.clone();

    // Run OrderBook management, applying sequenced updates to the local books
    tokio::spawn(book_manager.run());

    // Current OrderBook snapshots can now be accessed via the OrderBookMap
    // For example:
    let instrument_key = Instrument::new("btc", "usdt", InstrumentKind::Spot);
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!(%instrument_key, snapshot = ?books.find(&instrument_key).unwrap().read());
    tokio::time::sleep(Duration::from_secs(2)).await;
    info!(%instrument_key, snapshot = ?books.find(&instrument_key).unwrap().read());
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

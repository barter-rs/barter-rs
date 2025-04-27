use barter_data::books::{manager::init_indexed_multi_order_book_l2_manager, map::OrderBookMap};
use barter_instrument::{
    Underlying,
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{Instrument, InstrumentIndex},
};
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Creating Indexed Instruments with the Indexed Instruments Builder
    let indexed_instruments = IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::Coinbase,
            "coinbase_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            None,
        ))
        .build();

    let book_manager = init_indexed_multi_order_book_l2_manager(indexed_instruments.clone())
        .await
        .unwrap();

    // Clone OrderBookMap so you can access the locally managed OrderBooks elsewhere in your program
    let books = book_manager.books.clone();

    // Run OrderBook management, applying sequenced updates to the local books
    tokio::spawn(book_manager.run());

    // Current OrderBook snapshots can now be accessed via the OrderBookMap
    // You can retrieve an OrderBook either by using the InstrumentIndex,
    // or alternatively by using the InstrumentNameInternal.
    // For example:
    let instrument_key: InstrumentIndex = InstrumentIndex::new(1);
    let instrument_name_internal = indexed_instruments
        .find_instrument(instrument_key)
        .unwrap()
        .name_internal
        .clone();
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Access using InstrumentIndex
    info!(%instrument_key, snapshot = ?books.find(&instrument_key).unwrap().read());
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Access using InstrumentNameInternal
    info!(%instrument_name_internal, snapshot = ?books.instrument(&instrument_name_internal).read());
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

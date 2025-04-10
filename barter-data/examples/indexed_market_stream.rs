use barter_data::{
    streams::builder::dynamic::indexed::init_indexed_multi_exchange_market_stream,
    subscription::SubKind,
};
use barter_instrument::{
    Underlying,
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{
        Instrument,
        spec::{
            InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity,
            OrderQuantityUnits,
        },
    },
};
use futures_util::StreamExt;
use rust_decimal_macros::dec;
use tracing::info;

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise INFO Tracing log subscriber
    init_logging();

    // Construct IndexedInstruments
    let instruments = indexed_instruments();

    // Initialise indexed MarketStream:
    // - Uses IndexedInstruments to generate a Subscription for each Instrument-SubKind combination
    let mut stream = init_indexed_multi_exchange_market_stream(
        &instruments,
        &[SubKind::PublicTrades, SubKind::OrderBooksL1, SubKind::OrderBooksL2]
    )
    .await?;

    while let Some(event) = stream.next().await {
        info!("{event:?}");
    }

    Ok(())
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

fn indexed_instruments() -> IndexedInstruments {
    IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.0001), dec!(0.0)),
                InstrumentSpecQuantity::new(
                    OrderQuantityUnits::Quote,
                    dec!(0.00001),
                    dec!(0.00001),
                ),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.0001), dec!(0.0001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_sol_usdt",
            "SOLUSDT",
            Underlying::new("sol", "usdt"),
            Some(InstrumentSpec::new(
                InstrumentSpecPrice::new(dec!(0.01), dec!(0.01)),
                InstrumentSpecQuantity::new(OrderQuantityUnits::Quote, dec!(0.001), dec!(0.001)),
                InstrumentSpecNotional::new(dec!(5.0)),
            )),
        ))
        .build()
}

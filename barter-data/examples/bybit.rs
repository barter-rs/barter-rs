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
    init_logging();

    use ExchangeId::*;
    use InstrumentKind::*;
    use SubKind::*;


    let streams = DynamicStreams::init([
        vec![
            // (BybitPerpetualsUsd, "btc", "usdt", Perpetual, PublicTrades),
            // (BybitPerpetualsUsd, "btc", "usdt", Perpetual, OrderBooksL1),
            // (BybitPerpetualsUsd, "btc", "usdt", Perpetual, OrderBooksL2),
            (BybitPerpetualsUsd, "btc", "usdt", Perpetual, Liquidations),
        ],

        vec![
            // (BybitPerpetualsUsd, "eth", "usdt", Perpetual, PublicTrades),
            // (BybitPerpetualsUsd, "eth", "usdt", Perpetual, OrderBooksL1),
            // (BybitPerpetualsUsd, "eth", "usdt", Perpetual, OrderBooksL2),
            // (BybitPerpetualsUsd, "eth", "usdt", Perpetual, Liquidations),
        ],
    ]).await.unwrap();

    let mut merged = streams
        .select_all::<MarketEvent<Instrument, DataKind>>();

    while let Some(event) = merged.next().await {
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
        .init()
}

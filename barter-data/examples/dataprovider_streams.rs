#![cfg(feature = "databento")]

use std::error::Error;
use databento::dbn::{Dataset, SType, Schema};
use databento::live::Subscription;
use databento::LiveClient;
use futures_util::StreamExt;
use barter_data::provider::databento::DatabentoProvider;
use barter_data::provider::Provider;

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialise INFO Tracing log subscriber
    init_logging();

    let mut client = LiveClient::builder()
        .key_from_env()?
        .dataset(Dataset::DbeqBasic)
        .build()
        .await?;

    client.subscribe(
        Subscription::builder()
            .symbols(vec!["QQQ"])
            .schema(Schema::Mbo)
            .stype_in(SType::RawSymbol)
            .use_snapshot()
            .build(),
    ).await.unwrap();

    let mut provider = DatabentoProvider::new(client);
    let _ = provider.init().await?;
    while let Some(event) = provider.next().await {
        dbg!(event);
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

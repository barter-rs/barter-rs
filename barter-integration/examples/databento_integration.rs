use std::error::Error;
use databento::{
    dbn::{SType, Schema},
    live::Subscription,
    LiveClient,
};
use databento::dbn::{Dataset};
use futures::{StreamExt};
use barter_integration::protocol::StreamParser;
use barter_integration::stream::databento::DatabentoStream;
use barter_integration::Transformer;

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut client = LiveClient::builder()
        .key_from_env()?
        .dataset(Dataset::DbeqBasic)
        .build()
        .await?;

    client
        .subscribe(
            &Subscription::builder()
                .symbols(vec!["SPY", "NVDA", "MSFT", "PFE"])
                .schema(Schema::Mbo)
                .stype_in(SType::RawSymbol)
                .build(),
        )
        .await
        .unwrap();
    client.start().await?;

    let mut stream = DatabentoStream::new(client);

    while let Some(message) = stream.next().await {
        dbg!(message);
    }
    Ok(())
}


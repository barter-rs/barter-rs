use std::error::Error;
use databento::{
    dbn::{SType, Schema},
    live::Subscription,
    LiveClient,
};
use databento::dbn::{Dataset, PitSymbolMap};
use barter_integration::stream::databento::{DBTransformer, DatabentoStream};
use futures::{StreamExt};

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


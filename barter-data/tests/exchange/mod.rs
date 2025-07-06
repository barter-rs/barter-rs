use barter_data::{
    Identifier, MarketStream,
    error::DataError,
    exchange::{StreamSelector, binance::spot::BinanceSpot},
    instrument::InstrumentData,
    subscription::{Subscription, SubscriptionKind, trade::PublicTrades},
};
use barter_instrument::instrument::market_data::{
    MarketDataInstrument, kind::MarketDataInstrumentKind,
};
use barter_integration::{Validator, error::SocketError};
use futures_util::StreamExt;
use std::{collections::HashMap, hash::Hash, ops::Add};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn it_works() {
    init_logging();

    let test: MarketStreamTest<BinanceSpot, MarketDataInstrument, PublicTrades> =
        MarketStreamTest {
            timeout: std::time::Duration::from_secs(10),
            subscriptions: vec![Subscription::new(
                BinanceSpot::default(),
                MarketDataInstrument::new("btc", "usdt", MarketDataInstrumentKind::Spot),
                PublicTrades,
            )],
        };

    test.run().await.unwrap()
}

pub struct MarketStreamTest<Exchange, Instrument, Kind> {
    timeout: std::time::Duration,
    subscriptions: Vec<Subscription<Exchange, Instrument, Kind>>,
}

pub struct TestError<Exchange, Instrument, Kind> {
    test: MarketStreamTest<Exchange, Instrument, Kind>,
    kind: TestErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum TestErrorKind {
    #[error("test failed while setting up: {0}")]
    Setup(#[from] SocketError),

    #[error("test failed whilst initialising MarketStream: {0}")]
    Init(DataError),

    #[error("test encountered MarketStream error: {0}")]
    Stream(DataError),

    #[error("test failed due to early MarketStream termination")]
    StreamEnded,

    #[error("test failed to run within timeout duration")]
    Timeout,
}

impl<Exchange, Instrument, Kind> MarketStreamTest<Exchange, Instrument, Kind> {
    async fn run(self) -> Result<(), TestErrorKind>
    where
        Exchange: StreamSelector<Instrument, Kind>,
        Instrument: InstrumentData + Eq + Hash,
        Instrument::Key: Hash,
        Kind: SubscriptionKind,
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
    {
        // Validate Subscriptions
        let subscriptions = self
            .subscriptions
            .iter()
            .cloned()
            .map(Subscription::validate)
            .collect::<Result<Vec<_>, _>>()?;

        // Construct counter for MarketEvents per Instrument
        let mut num_events_per_instrument = subscriptions
            .iter()
            .map(|sub| (sub.instrument.key().clone(), 0))
            .collect::<HashMap<Instrument::Key, usize>>();

        let future = async move {
            let mut stream = Exchange::Stream::init::<Exchange::SnapFetcher>(&subscriptions)
                .await
                .map_err(TestErrorKind::Init)?;

            loop {
                if num_events_per_instrument.values().all(|count| *count >= 2) {
                    break Ok(());
                }

                let next = match stream.next().await {
                    Some(Ok(event)) => event,
                    Some(Err(error)) => break Err(TestErrorKind::Stream(error)),
                    None => break Err(TestErrorKind::StreamEnded),
                };

                println!("Consumed: {next:?}");

                *num_events_per_instrument.get_mut(&next.instrument).unwrap() += 1;
            }

            Ok(())
        };

        tokio::time::timeout(self.timeout, future)
            .await
            .map_err(|_| TestErrorKind::Timeout)?
    }
}

fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::layer())
        .init()
}

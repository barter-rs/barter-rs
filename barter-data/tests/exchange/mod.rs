use barter_data::{
    MarketStream,
    error::DataError,
    event::DataKind,
    instrument::InstrumentData,
    streams::{builder::dynamic::DynamicStreams, consumer::MarketStreamResult, reconnect::Event},
    subscription::{SubKind, Subscription, exchange_supports_instrument_kind_sub_kind},
};
use barter_instrument::{
    exchange::ExchangeId,
    instrument::market_data::{MarketDataInstrument, kind::MarketDataInstrumentKind},
};
use barter_integration::{Validator, error::SocketError};
use futures_util::StreamExt;
use std::{collections::HashMap, hash::Hash};
use strum::IntoEnumIterator;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn it_works() {
    init_logging();

    MarketStreamTest::builder(ExchangeId::BinanceSpot)
        .instruments([MarketDataInstrument::new(
            "btc",
            "usdt",
            MarketDataInstrumentKind::Spot,
        )])
        .build()
        .run()
        .await
        .unwrap()
}

#[derive(Debug, Clone)]
struct MarketStreamTest {
    timeout: std::time::Duration,
    subscriptions: Vec<Subscription>,
}

impl MarketStreamTest {
    fn builder(exchange: ExchangeId) -> MarketStreamTestBuilder {
        MarketStreamTestBuilder::new(exchange)
    }

    async fn run(self) -> Result<(), TestError> {
        // Construct counter for MarketEvents per Instrument
        let mut num_events_per_instrument = self
            .subscriptions
            .iter()
            .map(|sub| (sub.instrument.clone(), 0))
            .collect::<HashMap<_, _>>();

        let subscriptions = self.subscriptions.clone();
        let future = async move {
            let mut stream = DynamicStreams::init([subscriptions])
                .await
                .map_err(TestErrorKind::Init)?
                .select_all::<MarketStreamResult<MarketDataInstrument, DataKind>>();

            loop {
                if num_events_per_instrument.values().all(|count| *count >= 2) {
                    break Ok(());
                }

                let event = match stream.next().await {
                    Some(Event::Item(Ok(event))) => event,
                    Some(Event::Item(Err(error))) => break Err(TestErrorKind::Stream(error)),
                    Some(Event::Reconnecting(_)) | None => break Err(TestErrorKind::StreamEnded),
                };

                println!("Consumed: {event:?}");

                *num_events_per_instrument
                    .get_mut(&event.instrument)
                    .unwrap() += 1;
            }
        };

        tokio::time::timeout(self.timeout, future)
            .await
            .map_err(|_| TestError {
                test: self.clone(),
                kind: TestErrorKind::Timeout,
            })?
            .map_err(|_| TestError {
                test: self,
                kind: TestErrorKind::Timeout,
            })
    }
}

pub struct MarketStreamTestBuilder {
    exchange: ExchangeId,
    timeout: Option<std::time::Duration>,
    instruments: Vec<MarketDataInstrument>,
}

impl MarketStreamTestBuilder {
    fn new(exchange: ExchangeId) -> Self {
        Self {
            exchange,
            timeout: None,
            instruments: vec![],
        }
    }

    fn timeout(self, value: std::time::Duration) -> Self {
        Self {
            timeout: Some(value),
            ..self
        }
    }

    fn instruments<InstIter, Inst>(mut self, value: InstIter) -> Self
    where
        InstIter: IntoIterator<Item = Inst>,
        Inst: Into<MarketDataInstrument>,
    {
        self.instruments.extend(value.into_iter().map(Inst::into));
        self
    }

    fn build(self) -> MarketStreamTest {
        const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

        let Self {
            exchange,
            timeout,
            instruments,
        } = self;

        let timeout = timeout.unwrap_or(DEFAULT_TIMEOUT);

        if instruments.is_empty() {
            panic!("MarketStreamTestBuilder must be provided with MarketDataInstruments")
        }

        let subscriptions = instruments
            .into_iter()
            .flat_map(|instrument| {
                SubKind::iter().filter_map(move |sub_kind| {
                    exchange_supports_instrument_kind_sub_kind(
                        &exchange,
                        &instrument.kind,
                        sub_kind,
                    )
                    .then(|| Subscription::new(exchange, instrument.clone(), sub_kind))
                })
            })
            .collect();

        MarketStreamTest {
            timeout,
            subscriptions,
        }
    }
}

#[derive(Debug)]
pub struct TestError {
    test: MarketStreamTest,
    kind: TestErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum TestErrorKind {
    #[error("test failed whilst initialising MarketStream: {0}")]
    Init(DataError),

    #[error("test encountered MarketStream error: {0}")]
    Stream(DataError),

    #[error("test failed due to early MarketStream termination")]
    StreamEnded,

    #[error("test failed to run within timeout duration")]
    Timeout,
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

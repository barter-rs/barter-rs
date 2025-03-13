use std::error::Error;
use std::os::raw::c_char;
use std::pin::Pin;
use std::process::Output;
use std::task::{Context, Poll};
use std::time::Duration;
use chrono::DateTime;
use databento::dbn::{Dataset, MboMsg, Metadata, Record, RecordRef, SType, Schema, TradeMsg};
use databento::live::{Client, Subscription};
use databento::{Error as DbError, LiveClient};
use databento::dbn::Schema::Trades;
use futures::{pin_mut, Stream, TryFuture};
use futures_util::{stream, FutureExt, StreamExt};
use pin_project::pin_project;
use smol_str::ToSmolStr;
use tokio::sync::mpsc;
use tokio::time;
use barter_data::{
    event::DataKind,
    exchange::{
        binance::{futures::BinanceFuturesUsd, spot::BinanceSpot},
    },
    streams::{Streams, consumer::MarketStreamResult, reconnect::stream::ReconnectingStream},
    subscription::{
        book::{OrderBooksL1, OrderBooksL2},
        trade::PublicTrades,
    },
};
use barter_instrument::instrument::market_data::{
    MarketDataInstrument, kind::MarketDataInstrumentKind,
};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, warn};
use tracing::instrument::WithSubscriber;
use barter_data::books::{Level, OrderBook};
use barter_data::error::DataError;
use barter_data::event::MarketEvent;
use barter_data::streams::consumer::{StreamKey, STREAM_RECONNECTION_POLICY};
use barter_data::streams::reconnect::Event;
use barter_data::streams::reconnect::stream::init_reconnecting_stream;
use barter_data::subscription::book::OrderBookEvent;
use barter_data::subscription::SubscriptionKind;
use barter_data::subscription::trade::PublicTrade;
use barter_instrument::asset::name::AssetNameInternal;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::instrument::InstrumentIndex;
use barter_instrument::instrument::kind::InstrumentKind;
use barter_instrument::Side;
use barter_integration::protocol::websocket::{WebSocket, WsMessage};

trait CustomDataProvider {
    async fn init(&mut self) -> Result<(), impl Error>;
}

#[pin_project]
struct BentoDataProvider {
    #[pin]
    client: Client,
    initialised: bool,
}

impl BentoDataProvider {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            initialised: false,
        }
    }

}

pub fn transform_mb0(mbo: &MboMsg) -> Option<MarketEvent<InstrumentIndex, DataKind>> {
    let time_exchange = DateTime::from_timestamp_nanos(mbo.ts_recv as i64).to_utc();

    dbg!(mbo.flags.is_snapshot(), mbo.action);
    if mbo.flags.is_snapshot() && mbo.action.to_string() == "R" {
        return Some(MarketEvent {
            time_exchange: time_exchange.clone(),
            time_received: chrono::Utc::now(),
            exchange: ExchangeId::Other,
            instrument: InstrumentIndex(0),
            kind: DataKind::from(OrderBookEvent::Snapshot(
                OrderBook::new::<Vec<_>, Vec<_>, Level>(mbo.sequence as u64, Some(time_exchange), vec![], vec![])
            )),
        })
    }

    None
}

pub fn transform(record_ref: RecordRef) -> Option<MarketEvent<InstrumentIndex, DataKind>> {
    dbg!(record_ref);
    if let Some(mb0) = record_ref.get::<MboMsg>() {
        return transform_mb0(mb0);
    }
    // } else if let Some(trade) = record_ref.get::<TradeMsg>() {
    //     return transform_trade(trade);
    // }
    None
}

fn transform_trade(trade: &TradeMsg) -> MarketEvent<InstrumentIndex, DataKind> {
    let time_exchange = DateTime::from_timestamp_nanos(trade.ts_recv as i64).to_utc();
    MarketEvent {
        time_exchange: time_exchange.clone(),
        time_received: chrono::Utc::now(),
        exchange: ExchangeId::Other,
        instrument: InstrumentIndex(0),
        kind: DataKind::from(PublicTrade {
            id: "".to_string(),
            price: 0.0,
            amount: 0.0,
            side: Side::Buy,
        }),
    }
}

impl CustomDataProvider for BentoDataProvider {
    async fn init(&mut self) -> Result<(), DbError> {
        match self.client.start().await {
            Ok(_) => {
                self.initialised = true;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl Stream for BentoDataProvider {
    type Item = MarketStreamResult<InstrumentIndex, DataKind>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut binding = self.project();
        let future = binding.client.next_record();
        pin_mut!(future);

        let input = match future.try_poll(cx) {
            Poll::Ready(Ok(Some(record_ref))) => {
                transform(record_ref)
            },
            Poll::Pending => return Poll::Pending,
            _ => return Poll::Ready(None),
        };

        dbg!(&input);

        Poll::Pending

        // match input {
        //     None => {
        //         Poll::Pending
        //     },
        //     Some(event) => {
        //         Poll::Ready(Some(Event::Item(Ok(event))))
        //     }
        // }
    }
}

fn test_stream<S>(stream: S) -> S
where S: Stream<Item = MarketStreamResult<InstrumentIndex, DataKind>>{stream}

#[rustfmt::skip]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise INFO Tracing log subscriber
    init_logging();
    //
    // let mut stream =
    //     init_reconnecting_stream(||{
    //         async move {
    //             // let (tx, rx) = mpsc::channel::<Event<ExchangeId, MarketEvent>>(1);
    //             //
    //             // let events: Vec<MarketStreamResult<InstrumentIndex, DataKind>> = vec![
    //             //     Event::Item(Ok(
    //             //         MarketEvent {
    //             //             time_exchange: chrono::Utc::now(),
    //             //             time_received: chrono::Utc::now(),
    //             //             exchange: ExchangeId::Other,
    //             //             instrument: InstrumentIndex(0),
    //             //             kind: DataKind::from(PublicTrade {
    //             //                 id: "".to_string(),
    //             //                 price: 100.0,
    //             //                 amount: 1.0,
    //             //                 side: Side::Buy,
    //             //             }),
    //             //         }
    //             //     ))
    //             // ];
    //             // let stream = test_stream(stream::iter(events));
    //
    //             // tokio::spawn(async move {
    //             //     let mut interval = time::interval(Duration::from_secs(10));
    //             //      loop {
    //             //          interval.tick().await;
    //             //         let event = Event::Item(MarketEvent {
    //             //             time_exchange: chrono::Utc::now(),
    //             //             time_received: chrono::Utc::now(),
    //             //             exchange: ExchangeId::Other,
    //             //             instrument: MarketDataInstrument {
    //             //                 base: AssetNameInternal::from("btc"),
    //             //                 quote: AssetNameInternal::from("usdt"),
    //             //                 kind: MarketDataInstrumentKind::default(),
    //             //             },
    //             //             kind: DataKind::from(PublicTrade {
    //             //                 id: "".to_string(),
    //             //                 price: 100.0,
    //             //                 amount: 1.0,
    //             //                 side: Side::Buy,
    //             //             }),
    //             //         });
    //             //         if tx.send(event).await.is_err() {
    //             //             break;
    //             //         }
    //             //      }
    //             // });
    //             //
    //             // let stream = ReceiverStream::new(rx);
    //             let mut provider = BentoDataProvider {};
    //
    //         while let Some(event) = provider.next().await {
    //             dbg!(event);
    //         }
    //         }
    //     }).await?
    //      .with_reconnect_backoff(STREAM_RECONNECTION_POLICY, StreamKey::new("market_stream", ExchangeId::Other, Some("btc_usdt")));
    //
    // futures::pin_mut!(stream);

    // Notes:
    // - MarketStreamResult<_, DataKind> could use a custom enumeration if more flexibility is required.
    // - Each call to StreamBuilder::subscribe() creates a separate WebSocket connection for the
    //   Subscriptions passed.

    // while let Some(event) = stream.next().await {
    //     dbg!(event);
    // }

    let mut client = LiveClient::builder()
        .key_from_env()?
        .dataset(Dataset::DbeqBasic)
        .build()
        .await?;

    client.subscribe(
        Subscription::builder()
            .symbols(vec!["NVDA"])
            .schema(Schema::Mbo)
            .stype_in(SType::RawSymbol)
            .build(),
    ).await.unwrap();

    // client.subscribe(
    //   Subscription::builder()
    //     .symbols(vec!["QQQ"])
    //     .schema(Schema::Trades)
    //     .stype_in(SType::RawSymbol)
    //     .build(),
    // ).await.unwrap();

    let mut provider = BentoDataProvider::new(client);

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

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::error::Error;
use std::fmt::Display;
use std::os::raw::c_char;
use std::pin::Pin;
use std::process::Output;
use std::task::{Context, Poll};
use std::time::Duration;
use chrono::DateTime;
use databento::dbn::{Action, Dataset, MboMsg, Metadata, Record, RecordRef, SType, Schema, TradeMsg, Side as DbSide, SymbolIndex, pretty, UNDEF_PRICE, BidAskPair, Publisher};
use databento::live::{Client, Subscription};
use databento::{Error as DbError, HistoricalClient, LiveClient};
use databento::dbn::Schema::Trades;
use databento::historical::timeseries::{GetRangeParams, GetRangeToFileParams};
use futures::{pin_mut, Stream, TryFuture};
use futures_util::{stream, FutureExt, StreamExt};
use pin_project::pin_project;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use smol_str::ToSmolStr;
use time::macros::datetime;
use tokio::sync::mpsc;
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
use barter_data::provider::databento::DatabentoSide;
use barter_data::streams::consumer::{StreamKey, STREAM_RECONNECTION_POLICY};
use barter_data::streams::reconnect::Event;
use barter_data::streams::reconnect::stream::init_reconnecting_stream;
use barter_data::subscription::book::{OrderBookAction, OrderBookEvent, OrderBookUpdate};
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
struct DatabentoProvider {
    #[pin]
    client: Client,
    initialised: bool,
    buffer: VecDeque<MarketStreamResult<InstrumentIndex, DataKind>>,
}

impl DatabentoProvider {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            initialised: false,
            buffer: VecDeque::new(),
        }
    }
}

pub fn transform_mb0(mbo: &MboMsg) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {

    let time_exchange = DateTime::from_timestamp_nanos(mbo.ts_recv as i64).to_utc();

    // check if snapshot and create book here

    if mbo.price == UNDEF_PRICE {
        return Ok(None)
    }

    let side = mbo.side()?;
    let price = mbo.price_f64();

    match mbo.action() {
        Ok(Action::Add) => {
            Ok(Some(MarketEvent {
                time_exchange: time_exchange.clone(),
                time_received: chrono::Utc::now(),
                exchange: ExchangeId::Other,
                instrument: InstrumentIndex(0),
                kind: DataKind::from(OrderBookEvent::IncrementalUpdate(OrderBookUpdate {
                    order_id: Some(mbo.order_id.to_string()),
                    price: Decimal::from_f64(price).unwrap(),
                    amount: Decimal::from(mbo.size),
                    side: DatabentoSide::from(side).into(),
                    sequence: mbo.sequence as u64,
                    action: OrderBookAction::Add,
                })),
            }))
        }
        Ok(Action::Modify) => {
            Ok(Some(MarketEvent {
                time_exchange: time_exchange.clone(),
                time_received: chrono::Utc::now(),
                exchange: ExchangeId::Other,
                instrument: InstrumentIndex(0),
                kind: DataKind::from(OrderBookEvent::IncrementalUpdate(OrderBookUpdate {
                    order_id: Some(mbo.order_id.to_string()),
                    price: Decimal::from_f64(price).unwrap(),
                    amount: Decimal::from(mbo.size),
                    side: DatabentoSide::from(side).into(),
                    sequence: mbo.sequence as u64,
                    action: OrderBookAction::Modify,
                })),
            }))
        },
        Ok(Action::Cancel) => {
            Ok(Some(MarketEvent {
                time_exchange: time_exchange.clone(),
                time_received: chrono::Utc::now(),
                exchange: ExchangeId::Other,
                instrument: InstrumentIndex(0),
                kind: DataKind::from(OrderBookEvent::IncrementalUpdate(OrderBookUpdate {
                    order_id: Some(mbo.order_id.to_string()),
                    price: Decimal::from_f64(price).unwrap(),
                    amount: Decimal::from(mbo.size),
                    side: DatabentoSide::from(side).into(),
                    sequence: mbo.sequence as u64,
                    action: OrderBookAction::Cancel,
                })),
            }))
        },
        Ok(Action::Clear) => {
            Ok(Some(MarketEvent {
                time_exchange: time_exchange.clone(),
                time_received: chrono::Utc::now(),
                exchange: ExchangeId::Other,
                instrument: InstrumentIndex(0),
                kind: DataKind::from(OrderBookEvent::Clear),
            }))
        },
        Ok(Action::Trade) | Ok(Action::Fill) | Ok(Action::None) => {
            Ok(None)
        }
        Err(e) => {
            Err(DataError::from(e))
        }
    }
}

pub fn transform(record_ref: RecordRef) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {
    if let Some(mb0) = record_ref.get::<MboMsg>() {
        dbg!(mb0);
        return transform_mb0(mb0);
    }
    Ok(None)
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

impl CustomDataProvider for DatabentoProvider {
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

impl Stream for DatabentoProvider {
    type Item = MarketStreamResult<InstrumentIndex, DataKind>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            if let Some(output) = this.buffer.pop_front() {
                return Poll::Ready(Some(MarketStreamResult::from(output)));
            }

            let future = this.client.next_record();
            pin_mut!(future);

            let input = match future.try_poll(cx) {
                Poll::Ready(Ok(Some(record_ref))) => {
                    transform(record_ref)
                },
                Poll::Pending => return Poll::Pending,
                _ => {
                    return Poll::Ready(None)
                },
            };

            if input.is_err() {
                continue;
            }

            match input.unwrap() {
                Some(event) => {
                    this.buffer.push_back(Event::Item(Ok(event)));
                },
                None => {
                    continue;
                }
            }
        }
    }
}


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
    //
    // let mut client = HistoricalClient::builder()
    //     .key_from_env()?
    //     .build()?;

    // let path = "dbeq-basic-202505012.mbo.dbn.zst";

    // let mut decoder = client
    //     .timeseries()
    //     .get_range_to_file(
    //         &GetRangeToFileParams::builder()
    //             .dataset(Dataset::DbeqBasic)
    //             .date_time_range((
    //                 datetime!(2025-03-11 00:00:00 UTC),
    //                 datetime!(2025-03-13 23:00:00 UTC),
    //             ))
    //             .symbols("QQQ")
    //             .schema(Schema::Mbo)
    //             .path(path)
    //             .build(),
    //     )
    //     .await?;
    //
    // let symbol_map = decoder.metadata().symbol_map()?;

    // while let Some(mbo) = decoder.decode_record::<MboMsg>().await? {
    //     let result = transform_mb0(mbo);
    //     if mbo.flags.is_snapshot() {
    //         dbg!(mbo);
    //     }
    //
    //     ()
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

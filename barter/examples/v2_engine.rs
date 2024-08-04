// use barter::v2::{
//     channel::{mpsc_unbounded, Tx, UnboundedRx, UnboundedTx},
//     engine::{
//         audit::{Audit, AuditKind, Auditor},
//         error::{EngineError, ExecutionRxDropped},
//         state::{
//             balance::Balances,
//             instrument::{order::Orders, InstrumentState, Instruments},
//             EngineState, TradingState,
//         },
//         Engine,
//     },
//     execution,
//     execution::ExecutionRequest,
//     instrument::{
//         Instrument
//     },
//     position::Position,
//     risk::default::{DefaultRiskManager, DefaultRiskManagerState},
//     strategy::default::{DefaultStrategy, DefaultStrategyState},
//     EngineEvent,
// };
// use barter_data::{
//     event::MarketEvent,
//     instrument::{MarketInstrumentData},
//     streams::builder::dynamic::DynamicStreams,
//     subscription::{SubKind, Subscription},
// };
// use barter_instrument::{exchange::ExchangeId, instrument::kind::InstrumentKind, Keyed};
// use futures::{try_join, Stream};
// use std::{marker::PhantomData, time::Duration};
// use barter_instrument::asset::AssetId;
// use barter_instrument::instrument::InstrumentId;
// use barter_instrument::instrument::spec::{InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice, InstrumentSpecQuantity, OrderQuantityUnits};

#[tokio::main]
async fn main() {
    //     init_logging();
}
//     // Initialise channels
//     let (event_tx, mut event_rx, execution_tx, execution_rx, audit_tx, audit_rx) = init_channels();
//
//     // Construct Instrument definitions
//     let instruments = instruments();
//
//     // Initialise Market & Account links
//     let (mut market_link, mut account_link) = try_join!(
//         init_market_link(&instruments),
//         execution::link::init(execution_rx.into_stream(), &instruments)
//     )
//     .unwrap();
//
//     // JoinSet to spawn all async tasks
//     let mut join_set = tokio::task::JoinSet::new();
//
//     // Spawn task to send MarketEvents to Engine
//     join_set.spawn({
//         let event_tx = event_tx.clone();
//         async move {
//             while let Some(market) = market_link.next().await {
//                 if event_tx.send(EngineEvent::from(market)).is_err() {
//                     break;
//                 }
//             }
//         }
//     });
//
//     // Spawn task to send AccountEvents to Engine
//     join_set.spawn({
//         let event_tx = event_tx.clone();
//         async move {
//             while let Some(account) = account_link.next().await {
//                 if event_tx.send(EngineEvent::from(account)).is_err() {
//                     break;
//                 }
//             }
//         }
//     });
//
//     // Construct EngineState
//     let state = EngineState {
//         trading: TradingState::Disabled,
//         balances: Balances::default(),
//         instruments: instruments
//             .into_iter()
//             .map(|instrument| InstrumentState {
//                 market: MarketState::default(),
//                 orders: Orders::default(),
//                 position: Position::new_flat(instrument.key, "default"),
//                 instrument,
//             })
//             .collect(),
//         strategy: DefaultStrategyState,
//         risk: DefaultRiskManagerState,
//         phantom: PhantomData,
//     };
//
//     // // Spawn task to consume & log AuditEvents
//     join_set.spawn({
//         barter::v2::engine::audit::manager::run::<
//             _,
//             _,
//             DefaultStrategy,
//             _,
//             DefaultRiskManager,
//             _,
//             _,
//             _,
//             _,
//         >(state.clone(), audit_rx.into_stream())
//     });
//
//     let mut engine = Engine {
//         sequence: u64::MIN,
//         time: || chrono::Utc::now(),
//         execution_tx,
//         state: state.clone(),
//         strategy: DefaultStrategy,
//         risk: DefaultRiskManager,
//     };
//
//     // Run Engine
//     let task = join_set.spawn_blocking(move || {
//         let mut auditor = Auditor::new(audit_tx);
//         barter::v2::run(&mut event_rx, &mut auditor, &mut engine)
//         // .run_with_shutdown(|_| {
//         //     println!("shutting down Engine");
//         //     Ok(())
//         // }).unwrap();
//     });
//
//     tokio::time::sleep(Duration::from_secs(5)).await;
//     event_tx
//         .send(EngineEvent::TradingStateUpdate(TradingState::Enabled))
//         .unwrap();
//
//     join_set.join_all().await;
// }
//
// fn init_logging() {
//     tracing_subscriber::fmt()
//         // Filter messages based on the INFO
//         .with_env_filter(
//             tracing_subscriber::filter::EnvFilter::builder()
//                 .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
//                 .from_env_lossy(),
//         )
//         // Disable colours on release builds
//         .with_ansi(cfg!(debug_assertions))
//         // Enable Json formatting
//         .json()
//         // Install this Tracing subscriber as global default
//         .init()
// }
//
// fn init_channels() -> (
//     UnboundedTx<EngineEvent<AssetId, InstrumentId>, EngineError>,
//     UnboundedRx<EngineEvent<AssetId, InstrumentId>>,
//     UnboundedTx<ExecutionRequest<InstrumentId>, ExecutionRxDropped>,
//     UnboundedRx<ExecutionRequest<InstrumentId>>,
//     UnboundedTx<
//         Audit<
//             AuditKind<
//                 EngineState<
//                     Instruments<InstrumentId>,
//                     Balances<AssetId>,
//                     DefaultStrategyState,
//                     DefaultRiskManagerState,
//                     AssetId,
//                     InstrumentId,
//                 >,
//                 EngineEvent<AssetId, InstrumentId>,
//                 InstrumentId,
//                 EngineError,
//             >,
//         >,
//         EngineError,
//     >,
//     UnboundedRx<
//         Audit<
//             AuditKind<
//                 EngineState<
//                     Instruments<InstrumentId>,
//                     Balances<AssetId>,
//                     DefaultStrategyState,
//                     DefaultRiskManagerState,
//                     AssetId,
//                     InstrumentId,
//                 >,
//                 EngineEvent<AssetId, InstrumentId>,
//                 InstrumentId,
//                 EngineError,
//             >,
//         >,
//     >,
// ) {
//     let (event_tx, event_rx) = mpsc_unbounded();
//     let (execution_tx, execution_rx) = mpsc_unbounded();
//     let (audit_tx, audit_rx) = mpsc_unbounded();
//
//     (
//         event_tx,
//         event_rx,
//         execution_tx,
//         execution_rx,
//         audit_tx,
//         audit_rx,
//     )
// }
//
// fn instruments() -> Vec<Keyed<InstrumentId, Instrument<AssetId>>> {
//     vec![
//         Keyed {
//             key: InstrumentId(1),
//             value: Instrument {
//                 exchange: ExchangeId::BinanceSpot,
//                 name_exchange: "BTCUSDT".to_string(),
//                 kind: InstrumentKind::Spot,
//                 spec: InstrumentSpec {
//                     price: InstrumentSpecPrice {
//                         min: 0.0001,
//                         tick_size: 0.0,
//                     },
//                     quantity: InstrumentSpecQuantity {
//                         unit: OrderQuantityUnits::Quote,
//                         min: 0.00001,
//                         increment: 0.00001,
//                     },
//                     notional: InstrumentSpecNotional { min: 5.0 },
//                 },
//             }
//         },
//         Instrument {
//             key: InstrumentId(2),
//             exchange: ExchangeId::BinanceSpot,
//             name_exchange: "ETHUSDT".to_string(),
//             kind: InstrumentKind::Spot,
//             spec: InstrumentSpec {
//                 price: InstrumentSpecPrice {
//                     min: 0.01,
//                     tick_size: 0.01,
//                 },
//                 quantity: InstrumentSpecQuantity {
//                     unit: OrderQuantityUnits::Quote,
//                     min: 0.0001,
//                     increment: 0.0001,
//                 },
//                 notional: InstrumentSpecNotional { min: 5.0 },
//             },
//         },
//         Instrument {
//             key: InstrumentId(3),
//             exchange: ExchangeId::BinanceSpot,
//             name_exchange: "SOLUSDT".to_string(),
//             kind: InstrumentKind::Spot,
//             spec: InstrumentSpec {
//                 price: InstrumentSpecPrice {
//                     min: 0.01,
//                     tick_size: 0.01,
//                 },
//                 quantity: InstrumentSpecQuantity {
//                     unit: OrderQuantityUnits::Quote,
//                     min: 0.001,
//                     increment: 0.001,
//                 },
//                 notional: InstrumentSpecNotional { min: 5.0 },
//             },
//         },
//     ]
// }
//
// async fn init_market_link(
//     instruments: &[Instrument<InstrumentId>],
// ) -> Result<impl Stream<Item = MarketEvent<InstrumentId>>, EngineError> {
//     // OrderBookL1 subscription batches (ie/ Iterator<Item = [Subscription]>)
//     //  '-> this example uses a batch/websocket connection for each instrument OrderBookL1
//     let l1_subscription_batches = instruments.into_iter().map(|instrument| {
//         [Subscription::<ExchangeId, MarketInstrumentData>::new(
//             instrument.exchange,
//             MarketInstrumentData {
//                 id: instrument.key,
//                 name_exchange: instrument.name_exchange.clone(),
//                 kind: instrument.kind,
//             },
//             SubKind::OrderBooksL1,
//         )]
//     });
//
//     DynamicStreams::init(l1_subscription_batches)
//         .await
//         .map(DynamicStreams::select_all)
//         .map_err(EngineError::from)
// }

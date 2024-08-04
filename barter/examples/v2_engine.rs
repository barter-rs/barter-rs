use barter::v2::{
    channel::{mpsc_unbounded, Tx, UnboundedRx, UnboundedTx},
    engine::{
        audit::{AuditEvent, AuditEventKind, Auditor},
        error::EngineError,
        state::{
            balance::Balances,
            instrument::{market_data::MarketState, order::Orders, InstrumentState, Instruments},
            DefaultEngineState,
        },
        Engine,
    },
    execution,
    execution::ExecutionRequest,
    instrument::{
        Instrument, InstrumentSpec, InstrumentSpecNotional, InstrumentSpecPrice,
        InstrumentSpecQuantity, OrderQuantityUnits,
    },
    position::{PortfolioId, Position},
    risk::default::{DefaultRiskManager, DefaultRiskManagerState},
    strategy::{DefaultStrategy, DefaultStrategyState},
    EngineEvent, TryUpdater,
};
use barter_data::{
    event::MarketEvent,
    exchange::ExchangeId,
    instrument::{InstrumentId, MarketInstrumentData},
    streams::builder::dynamic::DynamicStreams,
    subscription::{book::OrderBookL1, SubKind, Subscription},
};
use barter_integration::model::{instrument::kind::InstrumentKind, Side};
use futures::{try_join, Stream, StreamExt};
use tracing::info;

#[tokio::main]
async fn main() {
    init_logging();

    // Initialise channels
    let (event_tx, event_rx, execution_tx, execution_rx, audit_tx, audit_rx) = init_channels();

    // Construct Instrument definitions
    let instruments = instruments();

    // Initialise Market & Account links
    let (mut market_link, mut account_link) = try_join!(
        init_market_link(&instruments),
        execution::link::init(execution_rx.into_stream(), &instruments)
    )
    .unwrap();

    // JoinSet to spawn all async tasks
    let mut join_set = tokio::task::JoinSet::new();

    // Spawn task to send MarketEvents to Engine
    join_set.spawn({
        let event_tx = event_tx.clone();
        async move {
            while let Some(market) = market_link.next().await {
                if event_tx.send(EngineEvent::from(market)).is_err() {
                    break;
                }
            }
        }
    });

    // Spawn task to send AccountEvents to Engine
    join_set.spawn({
        let event_tx = event_tx.clone();
        async move {
            while let Some(account) = account_link.next().await {
                if event_tx.send(EngineEvent::from(account)).is_err() {
                    break;
                }
            }
        }
    });

    // Construct EngineState
    let state = DefaultEngineState {
        balances: Balances::default(),
        instruments: instruments
            .into_iter()
            .map(|instrument| InstrumentState {
                market: MarketState::default(),
                orders: Orders::default(),
                position: Position::new_flat(instrument.id, "default"),
                instrument,
            })
            .collect(),
        strategy: DefaultStrategyState,
        risk: DefaultRiskManagerState,
    };

    // Spawn task to consume & log AuditEvents
    join_set.spawn({
        let mut audit_stream = audit_rx.into_stream();
        let mut state = state.clone();

        async move {
            while let Some(audit) = audit_stream.next().await {
                match audit.kind {
                    AuditEventKind::Snapshot(snapshot) => {
                        let _ = std::mem::replace(&mut state, snapshot);
                    }
                    AuditEventKind::Update {
                        input,
                        cancels,
                        opens,
                        refused_cancels,
                        refused_opens,
                    } => {
                        info!(event = ?input, "Engine received event");
                        state.try_update(&input).unwrap();

                        if !cancels.is_empty() {
                            info!(?cancels, "Engine generated risk approved cancel requests")
                        }
                        if !opens.is_empty() {
                            info!(?opens, "Engine generated risk approved open requests")
                        }
                        if !refused_cancels.is_empty() {
                            info!(
                                ?refused_cancels,
                                "Engine RiskManager refused cancel requests"
                            )
                        }
                        if !refused_opens.is_empty() {
                            info!(?refused_opens, "Engine RiskManager refused open requests")
                        }
                    }
                }
            }
        }
    });

    // Run Engine
    Engine {
        feed: event_rx,
        execution_tx,
        auditor: Auditor::new(audit_tx),
        state: state.clone(),
        strategy: DefaultStrategy,
        risk: DefaultRiskManager,
    }
    .run();
}

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

fn init_channels() -> (
    UnboundedTx<EngineEvent, EngineError>,
    UnboundedRx<EngineEvent>,
    UnboundedTx<ExecutionRequest<InstrumentId>, EngineError>,
    UnboundedRx<ExecutionRequest<InstrumentId>>,
    UnboundedTx<
        AuditEvent<
            DefaultEngineState<DefaultStrategyState, DefaultRiskManagerState>,
            EngineEvent,
            InstrumentId,
        >,
        EngineError,
    >,
    UnboundedRx<
        AuditEvent<
            DefaultEngineState<DefaultStrategyState, DefaultRiskManagerState>,
            EngineEvent,
            InstrumentId,
        >,
    >,
) {
    let (event_tx, event_rx) = mpsc_unbounded();
    let (execution_tx, execution_rx) = mpsc_unbounded();
    let (audit_tx, audit_rx) = mpsc_unbounded();

    (
        event_tx,
        event_rx,
        execution_tx,
        execution_rx,
        audit_tx,
        audit_rx,
    )
}

fn instruments() -> Vec<Instrument> {
    vec![
        Instrument {
            id: InstrumentId(1),
            exchange: ExchangeId::BinanceSpot,
            name_exchange: "BTCUSDT".to_string(),
            kind: InstrumentKind::Spot,
            spec: InstrumentSpec {
                price: InstrumentSpecPrice {
                    min: 0.0001,
                    tick_size: 0.0,
                },
                quantity: InstrumentSpecQuantity {
                    unit: OrderQuantityUnits::Quote,
                    min: 0.00001,
                    increment: 0.00001,
                },
                notional: InstrumentSpecNotional { min: 5.0 },
            },
        },
        Instrument {
            id: InstrumentId(2),
            exchange: ExchangeId::BinanceSpot,
            name_exchange: "ETHUSDT".to_string(),
            kind: InstrumentKind::Spot,
            spec: InstrumentSpec {
                price: InstrumentSpecPrice {
                    min: 0.01,
                    tick_size: 0.01,
                },
                quantity: InstrumentSpecQuantity {
                    unit: OrderQuantityUnits::Quote,
                    min: 0.0001,
                    increment: 0.0001,
                },
                notional: InstrumentSpecNotional { min: 5.0 },
            },
        },
        Instrument {
            id: InstrumentId(3),
            exchange: ExchangeId::BinanceSpot,
            name_exchange: "SOLUSDT".to_string(),
            kind: InstrumentKind::Spot,
            spec: InstrumentSpec {
                price: InstrumentSpecPrice {
                    min: 0.01,
                    tick_size: 0.01,
                },
                quantity: InstrumentSpecQuantity {
                    unit: OrderQuantityUnits::Quote,
                    min: 0.001,
                    increment: 0.001,
                },
                notional: InstrumentSpecNotional { min: 5.0 },
            },
        },
    ]
}

async fn init_market_link(
    instruments: &[Instrument],
) -> Result<impl Stream<Item = MarketEvent<InstrumentId>>, EngineError> {
    // OrderBookL1 subscription batches (ie/ Iterator<Item = [Subscription]>)
    //  '-> this example uses a batch/websocket connection for each instrument OrderBookL1
    let l1_subscription_batches = instruments.into_iter().map(|instrument| {
        [Subscription::<ExchangeId, MarketInstrumentData>::new(
            instrument.exchange,
            MarketInstrumentData {
                id: instrument.id,
                name_exchange: instrument.name_exchange.clone(),
                kind: instrument.kind,
            },
            SubKind::OrderBooksL1,
        )]
    });

    DynamicStreams::init(l1_subscription_batches)
        .await
        .map(DynamicStreams::select_all)
        .map_err(EngineError::from)
}

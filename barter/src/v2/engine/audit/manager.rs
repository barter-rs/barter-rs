use futures::{Stream, StreamExt};
use tracing::{error, info, warn};
use barter_data::event::MarketEvent;
use crate::v2::engine::audit::{Audit, AuditKind, GeneratedRequestsAudit, ShutdownAudit};
use crate::v2::engine::error::ExecutionRxDropped;
use crate::v2::engine::Processor;
use crate::v2::engine::state::{EngineState, TradingState};
use crate::v2::engine::state::instrument::OrderManager;
use crate::v2::EngineEvent;
use crate::v2::execution::{AccountEvent, AccountEventKind};
use crate::v2::order::{Order, RequestCancel, RequestOpen};
use crate::v2::risk::RiskManager;
use crate::v2::strategy::Strategy;

pub struct AuditSnapshot {

}

pub async fn run<State, AssetKey, InstrumentKey, StrategyT, Risk, Updates>(
    mut state: State,
    mut audit_stream: Updates,
)
where
    State: EngineState<AssetKey, InstrumentKey, StrategyT::State, Risk::State>
        + Processor<TradingState>
        + for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
        + for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
    StrategyT: Strategy<State, InstrumentKey>,
    Risk: RiskManager<State, InstrumentKey>,
    Updates: Stream<Item = Audit<AuditKind<State, EngineEvent<AssetKey, InstrumentKey>, InstrumentKey, ExecutionRxDropped>>> + Unpin
{
    let mut sequence = u64::MIN;
    while let Some(audit) = audit_stream.next().await {
        // Validate audit update sequence
        assert_eq!(sequence, audit.id);
        sequence += 1;

        // Update EngineState & log
        match audit.kind {
            AuditKind::Snapshot(snapshot) => {
                let _ = std::mem::replace(&mut state, snapshot);
                info!("Engine sent EngineState snapshot");
            }
            AuditKind::Process(event) => {
                update_state::<State, AssetKey, InstrumentKey, StrategyT, Risk>(&mut state, event);
            }
            AuditKind::ProcessWithGeneratedRequests(event, requests) => {
                update_state::<State, AssetKey, InstrumentKey, StrategyT, Risk>(&mut state, event);

                let order_manager = state.orders_mut();
                update_from_in_flight_cancel_requests(order_manager, &requests.cancels);
                update_from_in_flight_open_requests(order_manager, &requests.opens);
            }
            AuditKind::Shutdown(shutdown) => {
                log_shutdown_audit(shutdown)
            }
        }
    }
}

pub fn update_state<State, AssetKey, InstrumentKey, StrategyT, Risk>(
    state: &mut State,
    event: EngineEvent<AssetKey, InstrumentKey>,
)
where
    State: EngineState<AssetKey, InstrumentKey, StrategyT::State, Risk::State>
    + Processor<TradingState>
    + for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
    + for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
    StrategyT: Strategy<State, InstrumentKey>,
    Risk: RiskManager<State, InstrumentKey>,
{
    match event {
        EngineEvent::Shutdown => {
            // Todo: this will never hit, since it would be an AuditKind::Shutdown
            //  '--> shutdown back inside process(command)?
        },
        EngineEvent::TradingStateUpdate(trading_state) => {
            state.process(trading_state);
        }
        EngineEvent::Account(account) => {
            state.process(account);
        }
        EngineEvent::Market(market) => {
            state.process(market);
        }
        EngineEvent::Command(command) => {}
    }
}

pub fn update_from_in_flight_cancel_requests<InstrumentKey>(
    order_manager: &mut impl OrderManager<InstrumentKey>,
    cancels: &[Order<InstrumentKey, RequestCancel>],
)
{
    for request in cancels {
        order_manager.record_in_flight_cancel(request)
    }
}

pub fn update_from_in_flight_open_requests<InstrumentKey>(
    order_manager: &mut impl OrderManager<InstrumentKey>,
    opens: &[Order<InstrumentKey, RequestOpen>],
)
{
    for request in opens {
        order_manager.record_in_flight_open(request)
    }
}

pub fn log_engine_generated_requests<InstrumentKey>(
    requests: &GeneratedRequestsAudit<InstrumentKey>,
) {
    if !requests.cancels.is_empty() {
        info!(?requests.cancels, "Engine generated risk approved cancel requests")
    }
    if !requests.opens.is_empty() {
        info!(?requests.opens, "Engine generated risk approved open requests")
    }
    if !requests.refused_cancels.is_empty() {
        info!(?requests.refused_cancels, "Engine RiskManager refused cancel requests")
    }
    if !requests.refused_opens.is_empty() {
        info!(?requests.refused_opens, "Engine RiskManager refused open requests")
    }
}

pub fn log_shutdown_audit<Event, Error>(audit: ShutdownAudit<Event, Error>) {
    match audit {
        ShutdownAudit::FeedEnded => {
            info!("Engine shutdown to due input EventFeed ending");
        }
        ShutdownAudit::ExecutionEnded => {
            warn!("Engine shutdown to due ExecutionRx being dropped");
        }
        ShutdownAudit::AfterEvent(event) => {
            info!(?event, "Engine shutdown after processing event");
        }
        ShutdownAudit::WithError(event, error) => {
            error!(?event, ?error, "Engine shutdown after processing event generated an error")
        }
    }
}
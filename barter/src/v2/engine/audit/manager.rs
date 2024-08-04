// use crate::v2::engine::audit::{AuditKind, DefaultAudit, GeneratedRequestsAudit, ShutdownAudit};
// use crate::v2::engine::state::{EngineState, UpdateFromSnapshot};
// use crate::v2::engine::Processor;
// use crate::v2::execution::{AccountEvent, AccountEventKind, InstrumentAccountSnapshot};
// use crate::v2::order::{Order, RequestCancel, RequestOpen};
// use crate::v2::risk::RiskManager;
// use crate::v2::strategy::Strategy;
// use crate::v2::EngineEvent;
// use barter_data::event::MarketEvent;
// use futures::{Stream, StreamExt};
// use std::fmt::Debug;
// use tracing::{error, info, warn};
// use crate::v2::engine::state::balance::BalanceManager;
// use crate::v2::engine::state::instrument::market_data::MarketDataManager;
// use crate::v2::engine::state::instrument::order::OrderManager;
// use crate::v2::engine::state::instrument::position::PositionManager;
//
// pub async fn run<InstrumentState, BalanceState, StrategyT, StrategyState, Risk, RiskState, Updates, AssetKey, InstrumentKey>(
//     mut state: EngineState<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey>,
//     mut audit_stream: Updates,
// ) where
//     InstrumentState: UpdateFromSnapshot<Vec<InstrumentAccountSnapshot<InstrumentKey>>>
//         + MarketDataManager<InstrumentKey>
//         + OrderManager<InstrumentKey>
//         + PositionManager<InstrumentKey>,
//     BalanceState: BalanceManager<AssetKey>,
//     StrategyT: Strategy<InstrumentState, BalanceState, AssetKey, InstrumentKey, State = StrategyState, RiskState = RiskState>,
//     StrategyT::State: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
//         + for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
//     Risk: RiskManager<InstrumentState, BalanceState, AssetKey, InstrumentKey, State = RiskState, StrategyState = StrategyState>,
//     Risk::State: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
//         + for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
//     InstrumentKey: Debug + Clone,
//     AssetKey: Debug + Clone,
//     Updates: Stream<
//             Item = DefaultAudit<InstrumentState, BalanceState, StrategyState, RiskState, AssetKey, InstrumentKey, InstrumentState::Kind>,
//         > + Unpin,
// {
//     let mut sequence = u64::MIN;
//     while let Some(audit) = audit_stream.next().await {
//         // Validate audit update sequence
//         assert_eq!(sequence, audit.id);
//         sequence += 1;
//
//         // Update EngineState & log
//         match audit.kind {
//             AuditKind::Snapshot(snapshot) => {
//                 let _ = std::mem::replace(&mut state, snapshot);
//                 info!("Engine sent EngineState snapshot");
//             }
//             AuditKind::Process(event) => {
//                 update_state::<_, _, StrategyT, Risk, AssetKey, InstrumentKey>(&mut state, event);
//             }
//             AuditKind::ProcessWithGeneratedRequests(event, requests) => {
//                 update_state::<_, _, StrategyT, Risk, AssetKey, InstrumentKey>(&mut state, event);
//                 let order_manager = &mut state.instruments;
//                 update_from_in_flight_cancel_requests(order_manager, &requests.cancels);
//                 update_from_in_flight_open_requests(order_manager, &requests.opens);
//             }
//             AuditKind::Shutdown(shutdown) => log_shutdown_audit(shutdown),
//         }
//     }
// }
//
// pub fn update_state<InstrumentState, BalanceState, StrategyT, Risk, AssetKey, InstrumentKey>(
//     state: &mut EngineState<InstrumentState, BalanceState, StrategyT::State, Risk::State, AssetKey, InstrumentKey>,
//     event: EngineEvent<AssetKey, InstrumentKey, InstrumentState::Kind>,
// ) where
//     InstrumentState: UpdateFromSnapshot<Vec<InstrumentAccountSnapshot<InstrumentKey>>>
//         + MarketDataManager<InstrumentKey>
//         + OrderManager<InstrumentKey>
//         + PositionManager<InstrumentKey>,
//     BalanceState: BalanceManager<AssetKey>,
//     StrategyT: Strategy<InstrumentState, BalanceState, AssetKey, InstrumentKey>,
//     StrategyT::State: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
//         + for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
//     Risk: RiskManager<InstrumentState, BalanceState, AssetKey, InstrumentKey>,
//     Risk::State: for<'a> Processor<&'a AccountEvent<AccountEventKind<AssetKey, InstrumentKey>>>
//         + for<'a> Processor<&'a MarketEvent<InstrumentKey>>,
//     AssetKey: Debug + Clone,
//     InstrumentKey: Debug + Clone,
// {
//     match event {
//         EngineEvent::Shutdown => {
//             // Todo: this will never hit, since it would be an AuditKind::Shutdown
//             //  '--> shutdown back inside process(command)?
//         }
//         EngineEvent::TradingStateUpdate(trading_state) => {
//             state.process(trading_state);
//         }
//         EngineEvent::Account(account) => {
//             state.process(&account);
//         }
//         EngineEvent::Market(market) => {
//             state.process(&market);
//         }
//         EngineEvent::Command(_command) => {}
//     }
// }
//
// pub fn update_from_in_flight_cancel_requests<InstrumentKey: Clone>(
//     order_manager: &mut impl OrderManager<InstrumentKey>,
//     cancels: &[Order<InstrumentKey, RequestCancel>],
// ) {
//     for request in cancels {
//         order_manager.record_in_flight_cancel(request)
//     }
// }
//
// pub fn update_from_in_flight_open_requests<InstrumentKey: Clone>(
//     order_manager: &mut impl OrderManager<InstrumentKey>,
//     opens: &[Order<InstrumentKey, RequestOpen>],
// ) {
//     for request in opens {
//         order_manager.record_in_flight_open(request)
//     }
// }
//
// pub fn log_engine_generated_requests<InstrumentKey>(
//     requests: &GeneratedRequestsAudit<InstrumentKey>,
// ) where
//     InstrumentKey: Debug,
// {
//     if !requests.cancels.is_empty() {
//         info!(?requests.cancels, "Engine generated risk approved cancel requests")
//     }
//     if !requests.opens.is_empty() {
//         info!(?requests.opens, "Engine generated risk approved open requests")
//     }
//     if !requests.refused_cancels.is_empty() {
//         info!(?requests.refused_cancels, "Engine RiskManager refused cancel requests")
//     }
//     if !requests.refused_opens.is_empty() {
//         info!(?requests.refused_opens, "Engine RiskManager refused open requests")
//     }
// }
//
// pub fn log_shutdown_audit<Event, Error>(audit: ShutdownAudit<Event, Error>)
// where
//     Event: Debug,
//     Error: Debug,
// {
//     match audit {
//         ShutdownAudit::FeedEnded => {
//             info!("Engine shutdown to due input EventFeed ending");
//         }
//         ShutdownAudit::ExecutionEnded => {
//             warn!("Engine shutdown to due ExecutionRx being dropped");
//         }
//         ShutdownAudit::AfterEvent(event) => {
//             info!(?event, "Engine shutdown after processing event");
//         }
//         ShutdownAudit::WithError(event, error) => {
//             error!(
//                 ?event,
//                 ?error,
//                 "Engine shutdown after processing event generated an error"
//             )
//         }
//     }
// }

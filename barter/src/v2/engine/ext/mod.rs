// use std::fmt::Debug;
// use std::time::{Duration, Instant};
// use vecmap::{VecMap};
// use crate::v2::channel::Tx;
// use crate::v2::engine::{Engine};
// use crate::v2::engine::error::EngineError;
// use crate::v2::engine::state::balance::BalanceManager;
// use crate::v2::engine::state::instrument::{MarketDataManager, OrderManager, PositionManager};
//
// use crate::v2::execution::{AccountEvent, AccountEventKind, ExecutionRequest};
// use crate::v2::instrument::asset::AssetId;
// use crate::v2::order::{CancelRejectedReason, Cancelled, ClientOrderId, InternalOrderState, Order, OrderId, RequestCancel};
// use crate::v2::risk::RiskManager;
// use crate::v2::strategy::Strategy;
// use crate::v2::{EngineEvent, TryUpdater};
//
//
//
// // Todo: could we make this a CLI app? It should probably have a chatbot interface lol:
// //	"show me my orders" "filter by exchange"
// //   would benefit from extra integrations...
// // 	'--> Portfolio analysis
// //  	--> ask chatbot to analyse recent trades...
// //	--> use to access personal and/or public _financial information_
//
// // Todo: Trait useful for on_shutdown(), on_error() etc, which allows users to script functionality
// //   for various logic
// //  configured using a builder
//
// // Todo: I could just add to normal API of it's minimal
//
// // Todo: Rhai integration used to build "strategy" based on Engine actions and closures from traders
//
// pub trait EngineStateExt<Event, AssetKey, InstrumentKey, StrategyState, RiskState>
// where
//     Self: for<'a> TryUpdater<&'a Event> + Debug + Clone,
//     StrategyState: for<'a> TryUpdater<&'a Event> + Debug + Clone,
//     RiskState: for<'a> TryUpdater<&'a Event> + Debug + Clone,
// {
//     fn market_data(&self) -> &impl MarketDataManager<InstrumentKey>;
//     fn market_data_mut(&mut self) -> &mut impl MarketDataManager<InstrumentKey>;
//     fn balances(&self) -> &impl BalanceManager<AssetKey>;
//     fn balances_mut(&mut self) -> &mut impl BalanceManager<AssetKey>;
//     fn orders(&self) -> &impl OrderManagerExt<InstrumentKey>;
//     fn orders_mut(&mut self) -> &mut impl OrderManagerExt<InstrumentKey>;
//     fn positions(&self) -> &impl PositionManager<InstrumentKey>;
//     fn positions_mut(&mut self) -> &mut impl PositionManager<InstrumentKey>;
//     fn strategy(&self) -> &StrategyState;
//     fn strategy_mut(&mut self) -> &mut StrategyState;
//     fn risk(&self) -> &RiskState;
//     fn risk_mut(&mut self) -> &mut RiskState;
// }
//
// pub trait OrderManagerExt<InstrumentKey>
// where
//     Self: OrderManager<InstrumentKey>,
// {
//     fn orders<'a>(&'a self) -> impl Iterator<Item = &'a Order<InstrumentKey, InternalOrderState>>
//     where
//         InstrumentKey: 'a;
// }
//
// pub struct EngineCancelAllAudit<Error, InstrumentKey> {
//     error: Option<Error>,
//     results: VecMap<ClientOrderId, Order<InstrumentKey, Option<Result<Cancelled, Error>>>>,
// }
//
// impl<'a, Error, InstrumentKey> FromIterator<&'a Order<InstrumentKey, RequestCancel>> for EngineCancelAllAudit<Error, InstrumentKey>
// where
//     InstrumentKey: Clone,
// {
//     fn from_iter<T: IntoIterator<Item = &'a Order<InstrumentKey, RequestCancel>>>(iter: T) -> Self {
//         Self {
//             error: None,
//             results: iter
//                 .into_iter()
//                 .map(|order| {
//                     (order.cid, Order {
//                         instrument: order.instrument.clone(),
//                         cid: order.cid,
//                         side: order.side,
//                         state: None,
//                     })
//                 })
//                 .collect()
//         }
//     }
// }
//
// impl<EventFeed, ExecutionTx, AuditTx, State, StrategyT, Risk, InstrumentKey, Error> Engine<EventFeed, ExecutionTx, AuditTx, State, StrategyT, Risk>
// where
//     EventFeed: Iterator<Item = EngineEvent>,
//     ExecutionTx: Tx<Item = ExecutionRequest<InstrumentKey>, Error = EngineError>,
//     State: EngineStateExt<EngineEvent, AssetId, InstrumentKey, StrategyT::State, Risk::State>,
//     StrategyT: Strategy<State, Event = Error>,
//     Risk: RiskManager<State, Event = Error>,
//     InstrumentKey: Clone,
//     Error: From<EngineError>,
// {
//     // Todo: open_order, open_orders, recent_trades etc
//     fn cancel_all_orders(&mut self, timeout: Duration) -> EngineCancelAllAudit<Error, InstrumentKey> {
//         let start = Instant::now();
//
//         let order_manager = self
//             .state
//             .orders_mut();
//
//         // Generate cancels
//         let cancels = order_manager
//             .orders()
//             .into_iter()
//             .map(Order::<InstrumentKey, RequestCancel>::from)
//             .collect::<Vec<_>>();
//
//         // Construct CancelAll audit to keep track of results
//         let mut cancel_all_audit = EngineCancelAllAudit::from_iter(
//             &cancels
//         );
//
//         // Send to Execution link
//         if let Err(error) = self.execution_tx.send(ExecutionRequest::CancelOrders(cancels)) {
//             cancel_all_audit.error = Some(Error::from(error))
//         }
//
//         loop {
//             let event = match self.update_state_until(
//                 |event| matches!(event, EngineEvent::Account(AccountEvent { exchange: _, kind: AccountEventKind::OrderCancelled(_) })),
//                 timeout,
//             ) {
//                 Ok(Some(event)) => event,
//                 Ok(None) => {
//                     todo!()
//                 },
//                 Err(error) => {
//                     cancel_all_audit.error = Some(Error::from(error));
//                     return cancel_all_audit
//                 }
//             };
//
//             if let Err(error) = self.state.try_update(&event) {
//                 cancel_all_audit.error = Some(error);
//                 return cancel_all_audit
//             }
//
//             let EngineEvent::Account(AccountEvent { exchange: _, kind: AccountEventKind::OrderCancelled(order_response) }) = &event else {
//                 continue;
//             };
//
//             let Some(cancel) = cancel_all_audit.results.get_mut(&order_response.cid) else {
//                 continue;
//             };
//
//             cancel.state = Some(order_response.state.clone());
//
//             // Send EngineState audit update
//             self.auditor.audit(event, vec![], vec![], vec![], vec![]);
//
//             // Todo: Check if we received all the responses
//
//             let elapsed = Instant::now().saturating_duration_since(start);
//             if elapsed >= timeout {
//                 cancel_all_audit.error = Some(Error::from(EngineError::Timeout(timeout)));
//                 return cancel_all_audit;
//             }
//         }
//     }
//
//     fn update_state_until<FnUntil>(
//         &mut self,
//         until: FnUntil,
//         timeout: Duration,
//     ) -> Result<Option<EngineEvent>, EngineError>
//     where
//         FnUntil: Fn(&EngineEvent) -> bool,
//     {
//         let start = Instant::now();
//         for event in self.feed {
//             if until(&event) {
//                 return Ok(Some(event))
//             }
//
//             self.state.try_update(&event)?;
//             self.auditor.audit(event, vec![], vec![], vec![], vec![]);
//
//             let elapsed = Instant::now().saturating_duration_since(start);
//             if elapsed >= timeout {
//                 return Err(EngineError::Timeout(timeout))
//             }
//         }
//
//         Ok(None)
//     }
// }
//
// impl<'a, InstrumentKey> From<&'a Order<InstrumentKey, InternalOrderState>> for Order<InstrumentKey, RequestCancel>
// where
//     InstrumentKey: Clone,
// {
//     fn from(value: &'a Order<InstrumentKey, InternalOrderState>) -> Self {
//         Self {
//             instrument: value.instrument.clone(),
//             cid: value.cid,
//             side: value.side,
//             state: RequestCancel { id: value.state.order_id().unwrap_or_else(|| OrderId("unknown".to_string())) }
//         }
//     }
// }
//
// pub enum CancelOrderOutcome<InstrumentKey> {
//     CancelRejected(Order<InstrumentKey, CancelRejectedReason>),
//     Cancelled(Order<InstrumentKey, Cancelled>)
// }

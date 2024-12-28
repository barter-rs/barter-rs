use crate::engine::{
    action::{generate_algo_orders::GenerateAlgoOrdersOutput, ActionOutput},
    audit::{context::EngineContext, manager::AuditTick},
    state::position::PositionExited,
};
use barter_execution::trade::Trade;
use barter_instrument::asset::QuoteAsset;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct TradingHistory<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey> {
    states: Vec<AuditTick<State, EngineContext>>,
    commands: Vec<AuditTick<ActionOutput<ExchangeKey, InstrumentKey>, EngineContext>>,
    trading_disables: Vec<AuditTick<OnDisable, EngineContext>>,
    disconnections: Vec<AuditTick<OnDisconnect, EngineContext>>,
    orders_sent:
        Vec<AuditTick<GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>, EngineContext>>,
    trades: Vec<AuditTick<Trade<QuoteAsset, InstrumentKey>, EngineContext>>,
    positions: Vec<AuditTick<PositionExited<QuoteAsset, InstrumentKey>, EngineContext>>,
}

impl<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
    From<AuditTick<State, EngineContext>>
    for TradingHistory<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn from(value: AuditTick<State, EngineContext>) -> Self {
        Self {
            states: vec![value],
            commands: vec![],
            trading_disables: vec![],
            disconnections: vec![],
            orders_sent: vec![],
            trades: vec![],
            positions: vec![],
        }
    }
}

impl<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey> Default
    for TradingHistory<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
{
    fn default() -> Self {
        Self {
            states: vec![],
            commands: vec![],
            trading_disables: vec![],
            disconnections: vec![],
            orders_sent: vec![],
            trades: vec![],
            positions: vec![],
        }
    }
}

impl<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
    TradingHistory<State, OnDisable, OnDisconnect, ExchangeKey, InstrumentKey>
{
    pub fn add_state_snapshot(&mut self, event: State, context: EngineContext) {
        self.states.push(AuditTick::new(event, context));
    }

    pub fn add_command_output(
        &mut self,
        event: ActionOutput<ExchangeKey, InstrumentKey>,
        context: EngineContext,
    ) {
        self.commands.push(AuditTick::new(event, context))
    }

    pub fn add_trading_disabled_output(&mut self, event: OnDisable, context: EngineContext) {
        self.trading_disables.push(AuditTick::new(event, context))
    }

    pub fn add_disconnection_output(&mut self, event: OnDisconnect, context: EngineContext) {
        self.disconnections.push(AuditTick::new(event, context))
    }

    pub fn add_orders_sent(
        &mut self,
        event: GenerateAlgoOrdersOutput<ExchangeKey, InstrumentKey>,
        context: EngineContext,
    ) {
        self.orders_sent.push(AuditTick::new(event, context))
    }

    pub fn add_trade(&mut self, event: Trade<QuoteAsset, InstrumentKey>, context: EngineContext) {
        self.trades.push(AuditTick::new(event, context))
    }

    pub fn add_position(
        &mut self,
        event: PositionExited<QuoteAsset, InstrumentKey>,
        context: EngineContext,
    ) {
        self.positions.push(AuditTick::new(event, context))
    }
}

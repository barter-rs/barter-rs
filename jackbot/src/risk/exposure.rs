use crate::engine::state::{EngineState, instrument::data::InstrumentDataState};
use crate::risk::{RiskManager, RiskApproved, RiskRefused};
use crate::engine::state::instrument::filter::InstrumentFilter;
use crate::engine::command::Command;
use jackbot_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use jackbot_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Configuration limits for exposure risk management.
#[derive(Debug, Clone)]
pub struct ExposureLimits {
    /// Maximum notional exposure allowed per underlying asset.
    pub max_notional_per_underlying: Decimal,
    /// Maximum allowed drawdown percentage (eg/ 0.1 => 10%).
    pub max_drawdown_percent: Decimal,
    /// Correlation exposure limits between instrument pairs.
    /// Map of `(InstrumentIndex, InstrumentIndex)` -> max combined exposure.
    pub correlation_limits: HashMap<(InstrumentIndex, InstrumentIndex), Decimal>,
}

impl Default for ExposureLimits {
    fn default() -> Self {
        Self {
            max_notional_per_underlying: Decimal::MAX,
            max_drawdown_percent: Decimal::MAX,
            correlation_limits: HashMap::new(),
        }
    }
}

/// RiskManager that tracks exposure across exchanges and instruments.
#[derive(Debug, Clone)]
pub struct ExposureRiskManager<State> {
    pub limits: ExposureLimits,
    phantom: PhantomData<State>,
}

impl<State> Default for ExposureRiskManager<State> {
    fn default() -> Self {
        Self {
            limits: ExposureLimits::default(),
            phantom: PhantomData,
        }
    }
}

impl<GlobalData, InstrumentData> RiskManager<ExchangeIndex, InstrumentIndex>
    for ExposureRiskManager<EngineState<GlobalData, InstrumentData>>
where
    InstrumentData: InstrumentDataState,
{
    type State = EngineState<GlobalData, InstrumentData>;

    fn check(
        &self,
        state: &Self::State,
        cancels: impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        opens: impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) -> (
        impl IntoIterator<Item = RiskApproved<OrderRequestCancel<ExchangeIndex, InstrumentIndex>>>,
        impl IntoIterator<Item = RiskApproved<OrderRequestOpen<ExchangeIndex, InstrumentIndex>>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestCancel<ExchangeIndex, InstrumentIndex>>>,
        impl IntoIterator<Item = RiskRefused<OrderRequestOpen<ExchangeIndex, InstrumentIndex>>>,
    ) {
        let mut exposures = current_exposures(state);
        let mut approved_opens = Vec::new();
        let mut refused_opens = Vec::new();

        for open in opens.into_iter() {
            let inst_state = state.instruments.instrument_index(&open.key.instrument);
            let price = open.state.price
                .or_else(|| inst_state.data.price())
                .unwrap_or(Decimal::ZERO);
            let notional = crate::risk::check::util::calculate_quote_notional(
                open.state.quantity,
                price,
                inst_state.instrument.kind.contract_size(),
            ).unwrap_or(Decimal::ZERO);
            let underlying = inst_state.instrument.underlying.base;
            let current = exposures.entry(underlying).or_insert(Decimal::ZERO);

            if *current + notional > self.limits.max_notional_per_underlying {
                refused_opens.push(RiskRefused::new(open, "exposure limit"));
                continue;
            }
            if exceeds_drawdown(inst_state.position.current.as_ref(), self.limits.max_drawdown_percent) {
                refused_opens.push(RiskRefused::new(open, "drawdown limit"));
                continue;
            }
            *current += notional;
            approved_opens.push(RiskApproved::new(open));
        }

        (
            cancels.into_iter().map(RiskApproved::new),
            approved_opens,
            std::iter::empty(),
            refused_opens,
        )
    }
}

/// Generate mitigation commands for any instruments breaching limits.
pub fn mitigation_actions<GlobalData, InstrumentData>(
    limits: &ExposureLimits,
    state: &EngineState<GlobalData, InstrumentData>,
) -> Vec<Command<ExchangeIndex, AssetIndex, InstrumentIndex>>
where
    InstrumentData: InstrumentDataState,
{
    let mut actions = Vec::new();
    for inst_state in state.instruments.instruments(&InstrumentFilter::None) {
        if exceeds_drawdown(inst_state.position.current.as_ref(), limits.max_drawdown_percent) {
            actions.push(Command::ClosePositions(InstrumentFilter::Instruments(jackbot_integration::collection::one_or_many::OneOrMany::One(inst_state.key))));
        }
    }
    actions
}

fn current_exposures<GlobalData, InstrumentData>(
    state: &EngineState<GlobalData, InstrumentData>,
) -> HashMap<AssetIndex, Decimal>
where
    InstrumentData: InstrumentDataState,
{
    let mut map = HashMap::new();
    for inst_state in state.instruments.instruments(&InstrumentFilter::None) {
        if let Some(pos) = &inst_state.position.current {
            if let Some(price) = inst_state.data.price() {
                if let Some(notional) = crate::risk::check::util::calculate_quote_notional(
                    pos.quantity_abs,
                    price,
                    inst_state.instrument.kind.contract_size(),
                ) {
                    let entry = map.entry(inst_state.instrument.underlying.base).or_insert(Decimal::ZERO);
                    *entry += notional;
                }
            }
        }
    }
    map
}

fn exceeds_drawdown(pos: Option<&crate::engine::state::position::Position<jackbot_instrument::asset::QuoteAsset, InstrumentIndex>>, limit: Decimal) -> bool {
    let Some(position) = pos else { return false; };
    let invested = position.quantity_abs_max * position.price_entry_average;
    if invested.is_zero() {
        return false;
    }
    let pnl = position.pnl_realised + position.pnl_unrealised;
    let dd = (-pnl) / invested;
    dd > limit
}

/// Generate a simple textual dashboard summarising exposures.
pub fn generate_dashboard<GlobalData, InstrumentData>(
    state: &EngineState<GlobalData, InstrumentData>,
) -> String
where
    InstrumentData: InstrumentDataState,
{
    let exposures = current_exposures(state);
    let mut lines = vec!["--- Exposure Dashboard ---".to_string()];
    for (asset, value) in exposures {
        lines.push(format!("{asset:?}: {value}"));
    }
    lines.join("\n")
}


use crate::engine::state::{EngineState, instrument::data::InstrumentDataState};
use crate::risk::{RiskManager, RiskApproved, RiskRefused};
use crate::engine::state::instrument::filter::InstrumentFilter;
use crate::engine::command::Command;
use jackbot_execution::order::request::{OrderRequestCancel, OrderRequestOpen, RequestOpen};
use jackbot_execution::order::id::{ClientOrderId, StrategyId, OrderKey};
use jackbot_execution::order::{OrderKind, TimeInForce};
use jackbot_integration::collection::one_or_many::OneOrMany;
use jackbot_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex, Side};
use jackbot_instrument::{asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use rust_decimal::Decimal;
use jackbot_risk::volatility::VolatilityScaler;
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
    /// Optional volatility scaler used to adjust limits.
    pub scaler: Option<jackbot_risk::volatility::VolatilityScaler>,
    /// Per instrument volatility values.
    pub volatilities: HashMap<InstrumentIndex, Decimal>,
    phantom: PhantomData<State>,
}

impl<State> Default for ExposureRiskManager<State> {
    fn default() -> Self {
        Self {
            limits: ExposureLimits::default(),
            scaler: None,
            volatilities: HashMap::new(),
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

            let mut limit = self.limits.max_notional_per_underlying;
            if let Some(scaler) = &self.scaler {
                let vol = self
                    .volatilities
                    .get(&open.key.instrument)
                    .copied()
                    .unwrap_or(scaler.base_volatility);
                limit = scaler.adjust_risk(limit, vol);
            }

            if *current + notional > limit {
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

    let exposures_by_underlying = current_exposures(state);
    let exposures_by_instrument = current_instrument_exposures(state);
    let signed_exposures = current_instrument_signed_exposures(state);

    // Drawdown mitigation - fully close positions breaching the limit
    for inst_state in state.instruments.instruments(&InstrumentFilter::None) {
        if exceeds_drawdown(inst_state.position.current.as_ref(), limits.max_drawdown_percent) {
            actions.push(Command::ClosePositions(InstrumentFilter::Instruments(OneOrMany::One(inst_state.key))));
        }
    }

    // Exposure mitigation - partially reduce exposure to within limits
    for (asset, total) in exposures_by_underlying {
        if total > limits.max_notional_per_underlying {
            let mut excess = total - limits.max_notional_per_underlying;
            let mut instruments: Vec<_> = state
                .instruments
                .instruments(&InstrumentFilter::None)
                .filter(|s| s.instrument.underlying.base == asset && s.position.current.is_some())
                .collect();
            instruments.sort_by(|a, b| {
                exposures_by_instrument
                    .get(&b.key)
                    .unwrap_or(&Decimal::ZERO)
                    .cmp(exposures_by_instrument.get(&a.key).unwrap_or(&Decimal::ZERO))
            });

            for inst_state in instruments {
                if excess <= Decimal::ZERO {
                    break;
                }
                let notional = *exposures_by_instrument.get(&inst_state.key).unwrap_or(&Decimal::ZERO);
                let reduce = if notional >= excess { excess } else { notional };
                if let (Some(pos), Some(price)) = (&inst_state.position.current, inst_state.data.price()) {
                    let qty = reduce / (price * inst_state.instrument.kind.contract_size());
                    if qty > Decimal::ZERO {
                        let side = match pos.side {
                            Side::Buy => Side::Sell,
                            Side::Sell => Side::Buy,
                        };
                        let order = OrderRequestOpen {
                            key: OrderKey {
                                exchange: inst_state.instrument.exchange,
                                instrument: inst_state.key,
                                strategy: StrategyId::new("risk_mitigation"),
                                cid: ClientOrderId::random(),
                            },
                            state: RequestOpen {
                                side,
                                price,
                                quantity: qty,
                                kind: OrderKind::Market,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                            },
                        };
                        actions.push(Command::SendOpenRequests(OneOrMany::One(order)));
                        excess -= reduce;
                    }
                }
            }
        }
    }

    // Correlation mitigation - hedge exposures when pair limits are breached
    for ((a, b), limit) in &limits.correlation_limits {
        let exp_a = signed_exposures.get(a).copied().unwrap_or(Decimal::ZERO);
        let exp_b = signed_exposures.get(b).copied().unwrap_or(Decimal::ZERO);
        let combined = exp_a.abs() + exp_b.abs();
        if combined > *limit {
            let (hedge_key, hedge_exp) = if exp_a.abs() >= exp_b.abs() { (*a, exp_a) } else { (*b, exp_b) };
            let inst_state = state.instruments.instrument_index(&hedge_key);
            if let (Some(pos), Some(price)) = (&inst_state.position.current, inst_state.data.price()) {
                    let reduce = (combined - *limit).min(hedge_exp.abs());
                    let qty = reduce / (price * inst_state.instrument.kind.contract_size());
                    if qty > Decimal::ZERO {
                        let side = if hedge_exp > Decimal::ZERO { Side::Sell } else { Side::Buy };
                        let order = OrderRequestOpen {
                            key: OrderKey {
                                exchange: inst_state.instrument.exchange,
                                instrument: inst_state.key,
                                strategy: StrategyId::new("risk_mitigation"),
                                cid: ClientOrderId::random(),
                            },
                            state: RequestOpen {
                                side,
                                price,
                                quantity: qty,
                                kind: OrderKind::Market,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                            },
                        };
                        actions.push(Command::SendOpenRequests(OneOrMany::One(order)));
                    }
                }
            }
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

fn current_instrument_exposures<GlobalData, InstrumentData>(
    state: &EngineState<GlobalData, InstrumentData>,
) -> HashMap<InstrumentIndex, Decimal>
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
                    map.insert(inst_state.key, notional);
                }
            }
        }
    }
    map
}

fn current_instrument_signed_exposures<GlobalData, InstrumentData>(
    state: &EngineState<GlobalData, InstrumentData>,
) -> HashMap<InstrumentIndex, Decimal>
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
                    let signed = match pos.side {
                        Side::Buy => notional,
                        Side::Sell => -notional,
                    };
                    map.insert(inst_state.key, signed);
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

fn current_positions<GlobalData, InstrumentData>(
    state: &EngineState<GlobalData, InstrumentData>,
) -> HashMap<InstrumentIndex, Decimal>
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
                    map.insert(inst_state.key, notional);
                }
            }
        }
    }
    map
}

/// Generate a textual dashboard summarising positions, exposures, and alerts.
pub fn generate_dashboard<GlobalData, InstrumentData>(
    state: &EngineState<GlobalData, InstrumentData>,
    alerts: &[jackbot_risk::alert::RiskViolation<InstrumentIndex>],
) -> String
where
    InstrumentData: InstrumentDataState,
{
    let exposures = current_exposures(state);
    let positions = current_positions(state);

    let mut lines = vec!["--- Risk Dashboard ---".to_string()];
    if !positions.is_empty() {
        lines.push("Positions:".to_string());
        for (inst, value) in positions {
            lines.push(format!("{inst:?}: {value}"));
        }
    }

    if !exposures.is_empty() {
        lines.push("Exposure:".to_string());
        for (asset, value) in exposures {
            lines.push(format!("{asset:?}: {value}"));
        }
    }

    if !alerts.is_empty() {
        lines.push("Alerts:".to_string());
        for alert in alerts {
            lines.push(format!("{alert:?}"));
        }
    }

    lines.join("\n")
}


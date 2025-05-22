use crate::engine::state::{EngineState, instrument::data::InstrumentDataState, instrument::filter::InstrumentFilter};
use crate::engine::state::position::calculate_pnl_unrealised;
use jackbot_instrument::instrument::InstrumentIndex;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Utilities for stress testing portfolios by applying price shocks.
pub fn stress_test_pnl<GlobalData, InstrumentData>(
    state: &EngineState<GlobalData, InstrumentData>,
    pct_move: Decimal,
) -> HashMap<InstrumentIndex, Decimal>
where
    InstrumentData: InstrumentDataState,
{
    let mut pnl = HashMap::new();
    for inst_state in state.instruments.instruments(&InstrumentFilter::None) {
        if let Some(pos) = &inst_state.position.current {
            if let Some(price) = inst_state.data.price() {
                let new_price = price * (Decimal::ONE + pct_move);
                let unreal = calculate_pnl_unrealised(
                    pos.side,
                    pos.price_entry_average,
                    pos.quantity_abs,
                    pos.quantity_abs_max,
                    pos.fees_enter.fees,
                    new_price,
                );
                pnl.insert(inst_state.key, pos.pnl_realised + unreal);
            }
        }
    }
    pnl
}

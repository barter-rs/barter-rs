use crate::{
    engine::state::position::{calculate_pnl_return, PositionExited},
    statistic::summary::dataset::DataSetSummary,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct PnLReturns {
    /// Raw PnL.
    ///
    /// For Instrument PnL, this is most likely denominated in "quote asset" units. For example,
    /// btc_usdt  spot PnL would be in usdt. However, in some derivative cases the
    /// "settlement asset" could be different from the "quote asset.
    ///
    /// For Portfolio and Strategy PnL, this could be denominated in any asset chosen to aggregate
    /// PnL across different instruments.
    pub pnl_raw: f64,

    /// PnL returns statistical summary for wins and losses.
    pub total: DataSetSummary,

    /// PnL returns statistical summary for losses only.
    pub losses: DataSetSummary,
}

impl PnLReturns {
    pub fn update<AssetKey, InstrumentKey>(
        &mut self,
        position: &PositionExited<AssetKey, InstrumentKey>,
    ) {
        self.pnl_raw += position.pnl_realised;

        let pnl_return = calculate_pnl_return(
            position.pnl_realised,
            position.price_entry_average,
            position.quantity_abs_max,
        );

        self.total.update(pnl_return);

        if pnl_return.is_sign_negative() {
            self.losses.update(pnl_return)
        }
    }
}

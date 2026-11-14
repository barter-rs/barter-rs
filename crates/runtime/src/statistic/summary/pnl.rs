use crate::{
    engine::state::position::{PositionExited, calculate_pnl_return},
    statistic::summary::dataset::DataSetSummary,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Records Profit and Loss (PnL) data.
///
/// Includes tracking of:
/// - Raw PnL.
/// - Statistical summaries of returns for all closed positions (wins and losses combined)
/// - Statistical summaries of returns for all losing closed positions (useful for downside risk analysis).
///
/// # Asset Denomination
/// The raw PnL values can be denominated in different assets depending on the context:
/// - For Instrument PnL:
///   - Usually denominated in the quote asset (e.g., USDT for BTC-USDT spot)
///   - For derivatives, may be in the settlement asset if different from quote
/// - For Portfolio/Strategy PnL:
///   - Can be denominated in any chosen asset for cross-instrument aggregation
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
    pub pnl_raw: Decimal,

    /// PnL returns statistical summary for wins and losses.
    pub total: DataSetSummary,

    /// PnL returns statistical summary for losses only.
    pub losses: DataSetSummary,
}

impl PnLReturns {
    /// Update the `PnLReturns` from the next [`PositionExited`].
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

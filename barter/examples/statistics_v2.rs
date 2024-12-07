use barter::v2::{position::PositionExited, trade::AssetFees};
use barter_instrument::{
    asset::name::AssetNameInternal, instrument::name::InstrumentNameInternal, Side,
};
use chrono::{DateTime, TimeDelta, Utc};
use std::ops::Add;

#[tokio::main]
async fn main() {
    // let mut summary = TradingSummary::init(summary::trading::Config {
    //     starting_equity: 0.0,
    //     trading_days_per_year: 365,
    //     risk_free_return: 4.0,
    // });
}

fn positions() -> Vec<PositionExited<AssetNameInternal, InstrumentNameInternal>> {
    vec![PositionExited {
        instrument: InstrumentNameInternal::new("btc_usdt"),
        side: Side::Buy,
        price_entry_average: 100.0,
        quantity_abs_max: 20.0,
        pnl_realised: 0.0,
        fees_enter: AssetFees {
            asset: None,
            fees: 5.0,
        },
        fees_exit: AssetFees {
            asset: None,
            fees: 5.0,
        },
        time_enter: DateTime::<Utc>::MIN_UTC,
        time_exit: DateTime::<Utc>::MIN_UTC.add(TimeDelta::days(1)),
        trades: vec![],
    }]
}

use barter_integration::Side;
use derive_more::{Constructor};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Position<InstrumentKey> {
    pub instrument: InstrumentKey,
    pub side: Side,
    pub quantity: f64,
    pub price_average: f64,
    pub pnl_unrealised: f64,
    pub pnl_realised: f64,
}

impl<InstrumentKey> Position<InstrumentKey> {
    pub fn new_flat<PKey>(instrument: InstrumentKey) -> Self {
        Self {
            instrument,
            side: Side::Buy,
            quantity: 0.0,
            price_average: 0.0,
            pnl_unrealised: 0.0,
            pnl_realised: 0.0,
        }
    }
}

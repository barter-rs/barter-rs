use crate::order::OrderId;
use barter_instrument::Side;
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::fmt::{Display, Formatter};

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From, Constructor,
)]
pub struct TradeId<T = SmolStr>(pub T);

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Trade<AssetKey, InstrumentKey> {
    pub id: TradeId,
    pub instrument: InstrumentKey,
    pub order_id: OrderId,
    pub time_exchange: DateTime<Utc>,
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    pub fees: AssetFees<AssetKey>,
}

impl<AssetKey, InstrumentKey> Trade<AssetKey, InstrumentKey> {
    pub fn value_quote(&self) -> f64 {
        self.price * self.quantity.abs()
    }
}

impl<AssetKey, InstrumentKey> Display for Trade<AssetKey, InstrumentKey>
where
    AssetKey: Display,
    InstrumentKey: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ instrument: {}, side: {}, price: {}, quantity: {}, time: {} }}",
            self.instrument, self.side, self.price, self.quantity, self.time_exchange
        )
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetFees<AssetKey> {
    pub asset: Option<AssetKey>,
    pub fees: f64,
}

impl<AssetKey> Default for AssetFees<AssetKey> {
    fn default() -> Self {
        Self {
            asset: None,
            fees: 0.0,
        }
    }
}

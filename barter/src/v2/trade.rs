use crate::v2::order::OrderId;
use barter_integration::Side;
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From, Constructor,
)]
pub struct TradeId<T = String>(pub T);

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

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct AssetFees<AssetKey> {
    pub asset: Option<AssetKey>,
    pub fees: f64,
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

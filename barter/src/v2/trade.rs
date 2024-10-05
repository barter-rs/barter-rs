use crate::v2::{instrument::asset::AssetId, order::OrderId};
use barter_data::instrument::InstrumentId;
use barter_integration::model::Side;
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From, Constructor,
)]
pub struct TradeId<T = String>(pub T);

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Trade<InstrumentKey = InstrumentId, AssetKey = AssetId> {
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
pub struct AssetFees<AssetKey = AssetId> {
    pub asset: Option<AssetKey>,
    pub fees: f64,
}

impl std::fmt::Display for Trade {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ instrument: {}, side: {}, price: {}, quantity: {}, time: {} }}",
            self.instrument, self.side, self.price, self.quantity, self.time_exchange
        )
    }
}

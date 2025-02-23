use crate::order::id::{OrderId, StrategyId};
use barter_instrument::{Side, asset::QuoteAsset};
use chrono::{DateTime, Utc};
use derive_more::{Constructor, From};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, From)]
pub struct TradeId<T = SmolStr>(pub T);

impl TradeId {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Self(SmolStr::new(id))
    }
}

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct Trade<AssetKey, InstrumentKey> {
    pub id: TradeId,
    pub order_id: OrderId,
    pub instrument: InstrumentKey,
    pub strategy: StrategyId,
    pub time_exchange: DateTime<Utc>,
    pub side: Side,
    pub price: Decimal,
    pub quantity: Decimal,
    pub fees: AssetFees<AssetKey>,
}

impl<AssetKey, InstrumentKey> Trade<AssetKey, InstrumentKey> {
    pub fn value_quote(&self) -> Decimal {
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

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct AssetFees<AssetKey> {
    pub asset: AssetKey,
    pub fees: Decimal,
}

impl AssetFees<QuoteAsset> {
    pub fn quote_fees(fees: Decimal) -> Self {
        Self {
            asset: QuoteAsset,
            fees,
        }
    }
}

impl Default for AssetFees<QuoteAsset> {
    fn default() -> Self {
        Self {
            asset: QuoteAsset,
            fees: Decimal::ZERO,
        }
    }
}

impl<AssetKey> Default for AssetFees<Option<AssetKey>> {
    fn default() -> Self {
        Self {
            asset: None,
            fees: Decimal::ZERO,
        }
    }
}

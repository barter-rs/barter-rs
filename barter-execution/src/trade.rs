use crate::order::id::{OrderId, StrategyId};
use barter_instrument::Side;
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
pub struct Trade<InstrumentKey> {
    pub id: TradeId,
    pub order_id: OrderId,
    pub instrument: InstrumentKey,
    pub strategy: StrategyId,
    pub time_exchange: DateTime<Utc>,
    pub side: Side,
    pub price: Decimal,
    pub quantity: Decimal,
    pub fees: AssetFees,
}

impl<InstrumentKey> Trade<InstrumentKey> {
    pub fn fee_quote(&self) -> Decimal {
        match self.fees {
            AssetFees::Base(amount) => amount * self.price,
            AssetFees::Quote(amount) => amount,
        }
    }

    pub fn value_quote(&self) -> Decimal {
        self.price * self.quantity.abs()
    }
}

impl<InstrumentKey> Display for Trade<InstrumentKey>
where
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum AssetFees {
    Base(Decimal),
    Quote(Decimal),
}

impl AssetFees {
    pub fn base(fees: Decimal) -> Self {
        Self::Base(fees)
    }

    pub fn quote(fees: Decimal) -> Self {
        Self::Quote(fees)
    }

    pub fn amount(&self) -> Decimal {
        match self {
            Self::Base(amount) | Self::Quote(amount) => *amount,
        }
    }
}

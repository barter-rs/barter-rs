use barter_instrument::Side;
use databento::dbn::Side as DbSide;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum DatabentoSide {
    #[serde(alias = "buy", alias = "BUY", alias = "b")]
    Buy,
    #[serde(alias = "sell", alias = "SELL", alias = "s")]
    Sell,
}


impl From<DbSide> for DatabentoSide {
    fn from(value: DbSide) -> Self {
        match value {
            DbSide::Bid => DatabentoSide::Buy,
            DbSide::Ask => DatabentoSide::Sell,
            _ => {
                panic!("Invalid side")
            }
        }
    }
}

impl Into<Side> for DatabentoSide {
    fn into(self) -> Side {
        match self {
            DatabentoSide::Buy => Side::Buy,
            DatabentoSide::Sell => Side::Sell,
        }
    }
}
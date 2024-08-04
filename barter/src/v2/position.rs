use barter_data::instrument::InstrumentId;
use barter_integration::model::Side;
use derive_more::{Constructor, Display, From};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display, From,
)]
pub struct PortfolioId<Id = String>(pub Id);

impl From<&str> for PortfolioId<String> {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, Constructor)]
pub struct Position<InstrumentKey = InstrumentId, PortfolioKey = PortfolioId<String>> {
    pub instrument: InstrumentKey,
    pub portfolio: PortfolioKey,
    pub side: Side,
    pub quantity: f64,
    pub price_average: f64,
    pub pnl_unrealised: f64,
    pub pnl_realised: f64,
}

impl<InstrumentKey, PortfolioKey> Position<InstrumentKey, PortfolioKey> {
    pub fn new_flat<PKey>(instrument: InstrumentKey, portfolio: PKey) -> Self
    where
        PKey: Into<PortfolioKey>,
    {
        Self {
            instrument,
            portfolio: portfolio.into(),
            side: Side::Buy,
            quantity: 0.0,
            price_average: 0.0,
            pnl_unrealised: 0.0,
            pnl_realised: 0.0,
        }
    }
}

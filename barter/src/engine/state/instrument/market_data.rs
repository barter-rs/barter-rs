use crate::{Timed, engine::Processor};
use barter_data::{
    event::{DataKind, MarketEvent},
    subscription::book::OrderBookL1,
};
use barter_instrument::instrument::InstrumentIndex;
use derive_more::Constructor;
use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Defines a state object for tracking and managing the market data state of an instrument.
///
/// Implementations must handle market event processing and logic for providing the latest
/// instrument price.
///
/// The trait enables users to provide their own instrument market data state, and the type of
/// [`MarketEvent`] that is required to update it.
///
/// For an example, see the [`DefaultMarketData`] implementation.
pub trait MarketDataState<InstrumentKey = InstrumentIndex>
where
    Self: Debug + Clone + Send + for<'a> Processor<&'a MarketEvent<InstrumentKey, Self::EventKind>>,
{
    /// [`MarketEvent<_, EventKind>`](MarketEvent) expected by this market data state.
    type EventKind: Debug + Clone + Send;

    /// Latest price for an instrument, if available.
    ///
    /// Return the latest market price for an instrument, if available.
    ///
    /// An instrument price could be derived in many ways, but some common examples include:
    /// - Most recent `PublicTrade` price.
    /// - Volume-weighted mid-price from an `OrderBookL1`.
    /// - Volume-weighted mid-price from an `OrderBookL2`.
    fn price(&self) -> Option<Decimal>;
}

/// Basic [`MarketDataState`] that tracks the [`OrderBookL1`] and last traded price for an
/// instrument.
///
/// Trading strategies may wish to maintain more data here, such as candles, indicators,
/// L2 book, etc.
#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize, Constructor,
)]
pub struct DefaultMarketData {
    pub l1: OrderBookL1,
    pub last_traded_price: Option<Timed<Decimal>>,
}

impl MarketDataState for DefaultMarketData {
    type EventKind = DataKind;

    fn price(&self) -> Option<Decimal> {
        self.l1
            .volume_weighed_mid_price()
            .or(self.last_traded_price.as_ref().map(|timed| timed.value))
    }
}

impl<InstrumentKey> Processor<&MarketEvent<InstrumentKey, DataKind>> for DefaultMarketData {
    type Audit = ();

    fn process(&mut self, event: &MarketEvent<InstrumentKey, DataKind>) -> Self::Audit {
        match &event.kind {
            DataKind::Trade(trade) => {
                if self
                    .last_traded_price
                    .as_ref()
                    .is_none_or(|price| price.time < event.time_exchange)
                {
                    if let Some(price) = Decimal::from_f64(trade.price) {
                        self.last_traded_price
                            .replace(Timed::new(price, event.time_exchange));
                    }
                }
            }
            DataKind::OrderBookL1(l1) => {
                if self.l1.last_update_time < event.time_exchange {
                    self.l1 = l1.clone()
                }
            }
            _ => {}
        }
    }
}

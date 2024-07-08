use super::Kraken;
use crate::{
    instrument::{KeyedInstrument, MarketInstrumentData},
    subscription::Subscription,
    Identifier,
};
use barter_integration::model::instrument::{symbol::Symbol, Instrument};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Kraken`] market that can be subscribed to.
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenMarket(pub String);

impl<Kind> Identifier<KrakenMarket> for Subscription<Kraken, Instrument, Kind> {
    fn id(&self) -> KrakenMarket {
        kraken_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<Kind> Identifier<KrakenMarket> for Subscription<Kraken, KeyedInstrument, Kind> {
    fn id(&self) -> KrakenMarket {
        kraken_market(&self.instrument.data.base, &self.instrument.data.quote)
    }
}

impl<Kind> Identifier<KrakenMarket> for Subscription<Kraken, MarketInstrumentData, Kind> {
    fn id(&self) -> KrakenMarket {
        KrakenMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for KrakenMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn kraken_market(base: &Symbol, quote: &Symbol) -> KrakenMarket {
    KrakenMarket(format!("{base}/{quote}").to_uppercase())
}

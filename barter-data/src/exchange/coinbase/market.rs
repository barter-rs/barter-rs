use super::Coinbase;
use crate::{
    instrument::{KeyedInstrument, MarketInstrumentData},
    subscription::Subscription,
    Identifier,
};
use barter_integration::model::instrument::{symbol::Symbol, Instrument};
use serde::{Deserialize, Serialize};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Coinbase`](super::Coinbase) market that can be subscribed to.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbaseMarket(pub String);

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, Instrument, Kind> {
    fn id(&self) -> CoinbaseMarket {
        coinbase_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, KeyedInstrument, Kind> {
    fn id(&self) -> CoinbaseMarket {
        coinbase_market(&self.instrument.data.base, &self.instrument.data.quote)
    }
}

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, MarketInstrumentData, Kind> {
    fn id(&self) -> CoinbaseMarket {
        CoinbaseMarket(self.instrument.name_exchange.clone())
    }
}

impl AsRef<str> for CoinbaseMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn coinbase_market(base: &Symbol, quote: &Symbol) -> CoinbaseMarket {
    CoinbaseMarket(format!("{base}-{quote}").to_uppercase())
}

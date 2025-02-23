use super::Coinbase;
use crate::{Identifier, instrument::MarketInstrumentData, subscription::Subscription};
use barter_instrument::{
    Keyed, asset::name::AssetNameInternal, instrument::market_data::MarketDataInstrument,
};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Coinbase`] market that can be subscribed to.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbaseMarket(pub SmolStr);

impl<Kind> Identifier<CoinbaseMarket> for Subscription<Coinbase, MarketDataInstrument, Kind> {
    fn id(&self) -> CoinbaseMarket {
        coinbase_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<CoinbaseMarket>
    for Subscription<Coinbase, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> CoinbaseMarket {
        coinbase_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<CoinbaseMarket>
    for Subscription<Coinbase, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> CoinbaseMarket {
        CoinbaseMarket(self.instrument.name_exchange.name().clone())
    }
}

impl AsRef<str> for CoinbaseMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn coinbase_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> CoinbaseMarket {
    CoinbaseMarket(format_smolstr!("{base}-{quote}").to_uppercase_smolstr())
}

use super::Bitfinex;
use crate::{Identifier, instrument::MarketInstrumentData, subscription::Subscription};
use barter_instrument::{
    Keyed, asset::name::AssetNameInternal, instrument::market_data::MarketDataInstrument,
};
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, ToSmolStr, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Bitfinex`] market that can be subscribed to.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-public>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexMarket(pub SmolStr);

impl<Kind> Identifier<BitfinexMarket> for Subscription<Bitfinex, MarketDataInstrument, Kind> {
    fn id(&self) -> BitfinexMarket {
        bitfinex_market(&self.instrument.base, &self.instrument.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<BitfinexMarket>
    for Subscription<Bitfinex, Keyed<InstrumentKey, MarketDataInstrument>, Kind>
{
    fn id(&self) -> BitfinexMarket {
        bitfinex_market(&self.instrument.value.base, &self.instrument.value.quote)
    }
}

impl<InstrumentKey, Kind> Identifier<BitfinexMarket>
    for Subscription<Bitfinex, MarketInstrumentData<InstrumentKey>, Kind>
{
    fn id(&self) -> BitfinexMarket {
        BitfinexMarket(self.instrument.name_exchange.to_smolstr())
    }
}

impl AsRef<str> for BitfinexMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn bitfinex_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> BitfinexMarket {
    BitfinexMarket(format_smolstr!(
        "t{}{}",
        base.to_string().to_uppercase(),
        quote.to_string().to_uppercase()
    ))
}

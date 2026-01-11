use super::Bitfinex;
use crate::impl_market_identifier;
use barter_instrument::asset::name::AssetNameInternal;
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Bitfinex`] market that can be subscribed to.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-public>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BitfinexMarket(pub SmolStr);

impl_market_identifier!(Bitfinex => BitfinexMarket, bitfinex_market);

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

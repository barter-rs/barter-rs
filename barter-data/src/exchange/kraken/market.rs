use super::Kraken;
use crate::impl_market_identifier;
use barter_instrument::asset::name::AssetNameInternal;
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Kraken`] market that can be subscribed to.
///
/// See docs: <https://docs.kraken.com/websockets/#message-subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct KrakenMarket(pub SmolStr);

impl_market_identifier!(Kraken => KrakenMarket, kraken_market);

impl AsRef<str> for KrakenMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn kraken_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> KrakenMarket {
    KrakenMarket(format_smolstr!("{base}/{quote}").to_lowercase_smolstr())
}

use super::Coinbase;
use crate::impl_market_identifier;
use barter_instrument::asset::name::AssetNameInternal;
use serde::{Deserialize, Serialize};
use smol_str::{SmolStr, StrExt, format_smolstr};

/// Type that defines how to translate a Barter [`Subscription`] into a
/// [`Coinbase`] market that can be subscribed to.
///
/// See docs: <https://docs.cloud.coinbase.com/exchange/docs/websocket-overview#subscribe>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct CoinbaseMarket(pub SmolStr);

impl_market_identifier!(Coinbase => CoinbaseMarket, coinbase_market);

impl AsRef<str> for CoinbaseMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn coinbase_market(base: &AssetNameInternal, quote: &AssetNameInternal) -> CoinbaseMarket {
    CoinbaseMarket(format_smolstr!("{base}-{quote}").to_uppercase_smolstr())
}

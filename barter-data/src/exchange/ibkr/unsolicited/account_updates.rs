use barter_integration::subscription::SubscriptionId;
use chrono::Utc;
use serde::Deserialize;

use crate::{
    event::{MarketEvent, MarketIter},
    exchange::ExchangeId,
    subscription::account::Account,
    Identifier,
};

/// ### Account Response
/// ```json
/// {
///   "topic":"act",
///   "args": {
///     "accounts": [
///       "abcd",
///       "efgh"
///     ],
///     "acctProps": {
///       "abcd": {},
///       "efgh": {}
///     },
///     "aliases": {
///       "abcd": "retirement",
///       "efgh": "gambling"
///     }
///   }
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IbkrAccountResponse {
    #[serde(rename = "args")]
    pub args: AccountArgs,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct AccountArgs {
    pub accounts: Vec<String>,
    pub selected_account: String,
}

impl Identifier<Option<SubscriptionId>> for IbkrAccountResponse {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(self.args.selected_account.clone()))
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, IbkrAccountResponse)>
    for MarketIter<InstrumentId, Account>
{
    fn from(
        (exchange_id, instrument, _account_update): (ExchangeId, InstrumentId, IbkrAccountResponse),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: Utc::now(),
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: Account {},
        })])
    }
}

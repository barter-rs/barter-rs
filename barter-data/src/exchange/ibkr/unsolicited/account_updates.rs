use barter_integration::model::{Exchange, SubscriptionId};
use chrono::Utc;
use serde::Deserialize;

use crate::{event::{MarketEvent, MarketIter}, exchange::ExchangeId, subscription::account::Account, Identifier};

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
pub struct IbkrAccount {
    #[serde(rename = "args")]
    args: AccountArgs,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct AccountArgs {
    accounts: Vec<String>,
    selected_account: String,
}

impl Identifier<Option<SubscriptionId>> for IbkrAccount {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(self.args.selected_account.clone()))
    }
}

impl<InstrumentId> From<(ExchangeId, InstrumentId, IbkrAccount)> for MarketIter<InstrumentId, Account> {
    fn from((exchange_id, instrument, _account_update): (ExchangeId, InstrumentId, IbkrAccount)) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_time: Utc::now(),
            received_time: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: Account {},
        })])
    }
}

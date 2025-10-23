use std::fmt::Debug;

use crate::{Identifier, exchange::bitstamp::channel::BitstampChannel};
use barter_integration::subscription::SubscriptionId;
use serde::{
    Deserialize, Serialize,
    de::{Error, Unexpected},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct BitstampPayload<T> {
    #[serde(alias = "channel", deserialize_with = "de_message_subscription_id")]
    pub subscription_id: SubscriptionId,

    pub data: T,
}

pub fn de_message_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let input = <&str as serde::Deserialize>::deserialize(deserializer)?;

    let (channel, market) = input
        .rsplit_once('_')
        .ok_or_else(|| D::Error::missing_field("market is expected"))?;

    match channel {
        "diff_order_book" => Ok(SubscriptionId::from(format!(
            "{}|{market}",
            BitstampChannel::ORDER_BOOK_L2.0,
        ))),
        _ => Err(Error::invalid_value(
            Unexpected::Str(input),
            &"invalid message type expected pattern: <type>_<symbol>",
        )),
    }
}

impl<T> Identifier<Option<SubscriptionId>> for BitstampPayload<T> {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

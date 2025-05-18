use std::fmt::Debug;

use crate::{Identifier, exchange::onetrading::channel::OneTradingChannel};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{
    Deserialize, Serialize,
    de::{Error, Unexpected},
};

/// Generic message payload for OneTrading WebSocket messages
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct OneTradingPayload<T> {
    /// Message type identifier
    #[serde(rename = "type")]
    pub kind: String,
    
    /// Channel name (e.g. PRICE_TICKS, ORDERBOOK, BOOK_TICKER)
    #[serde(alias = "channel", deserialize_with = "de_message_subscription_id")]
    pub subscription_id: SubscriptionId,
    
    /// Timestamp in nanoseconds
    #[serde(
        alias = "time",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    
    /// Message data
    pub data: T,
}

/// Deserialize a message channel and instrument into a SubscriptionId
///
/// e.g., channel: "PRICE_TICKS", instrument: "BTC_EUR" becomes "PRICE_TICKS|BTC_EUR"
pub fn de_message_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct ChannelInfo {
        name: String,
        instrument: String,
    }

    let channel_info = ChannelInfo::deserialize(deserializer)?;
    
    // Map the channel name to our internal channel constants
    let channel_name = match channel_info.name.as_str() {
        "PRICE_TICKS" => OneTradingChannel::TRADES.0,
        "BOOK_TICKER" => OneTradingChannel::ORDER_BOOK_L1.0,
        "ORDERBOOK" => OneTradingChannel::ORDER_BOOK_L2.0,
        _ => {
            return Err(Error::invalid_value(
                Unexpected::Str(&channel_info.name),
                &"expected one of: PRICE_TICKS, BOOK_TICKER, ORDERBOOK",
            ))
        }
    };

    Ok(SubscriptionId::from(format!(
        "{}|{}",
        channel_name,
        channel_info.instrument
    )))
}

impl<T> Identifier<Option<SubscriptionId>> for OneTradingPayload<T> {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}
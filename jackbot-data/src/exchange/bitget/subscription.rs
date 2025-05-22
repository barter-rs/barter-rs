//! Subscription logic and types for Bitget exchange.

use crate::exchange::bitget::channel::BitgetChannel;
use jackbot_integration::{protocol::websocket::WsMessage, subscription::SubscriptionId};
use serde_json::json;
use std::collections::HashMap;

/// Subscription message for Bitget WebSocket
#[derive(Debug, Clone)]
pub struct BitgetSubscription {
    pub channel: BitgetChannel,
    pub inst_id: String,
}

impl BitgetSubscription {
    /// Create a new subscription for a specific channel and instrument
    pub fn new(channel: BitgetChannel, inst_id: String) -> Self {
        Self { channel, inst_id }
    }

    /// Create a subscription message for spot market
    pub fn spot_subscribe_message(&self) -> WsMessage {
        let msg = json!({
            "op": "subscribe",
            "args": [{
                "channel": self.channel.as_str(),
                "instId": self.inst_id
            }]
        });
        WsMessage::Text(msg.to_string())
    }

    /// Create a subscription message for futures market
    pub fn futures_subscribe_message(&self) -> WsMessage {
        let msg = json!({
            "op": "subscribe",
            "args": [{
                "channel": self.channel.as_str(),
                "instId": self.inst_id
            }]
        });
        WsMessage::Text(msg.to_string())
    }

    /// Create an unsubscribe message for spot market
    pub fn spot_unsubscribe_message(&self) -> WsMessage {
        let msg = json!({
            "op": "unsubscribe",
            "args": [{
                "channel": self.channel.as_str(),
                "instId": self.inst_id
            }]
        });
        WsMessage::Text(msg.to_string())
    }

    /// Create an unsubscribe message for futures market
    pub fn futures_unsubscribe_message(&self) -> WsMessage {
        let msg = json!({
            "op": "unsubscribe",
            "args": [{
                "channel": self.channel.as_str(),
                "instId": self.inst_id
            }]
        });
        WsMessage::Text(msg.to_string())
    }

    /// Convert subscription IDs to subscription messages (spot market)
    pub fn spot_subscription_messages(
        subscription_ids: &[SubscriptionId],
        id_map: &HashMap<SubscriptionId, BitgetSubscription>,
    ) -> Vec<WsMessage> {
        // Group subscriptions by channel
        let mut channel_groups: HashMap<&BitgetChannel, Vec<String>> = HashMap::new();

        for sub_id in subscription_ids {
            if let Some(subscription) = id_map.get(sub_id) {
                channel_groups
                    .entry(&subscription.channel)
                    .or_default()
                    .push(subscription.inst_id.clone());
            }
        }

        // Create subscription messages for each channel group
        channel_groups
            .iter()
            .map(|(channel, inst_ids)| {
                let args = inst_ids
                    .iter()
                    .map(|inst_id| {
                        json!({
                            "channel": channel.as_str(),
                            "instId": inst_id
                        })
                    })
                    .collect::<Vec<_>>();

                let msg = json!({
                    "op": "subscribe",
                    "args": args
                });
                WsMessage::Text(msg.to_string())
            })
            .collect()
    }

    /// Convert subscription IDs to subscription messages (futures market)
    pub fn futures_subscription_messages(
        subscription_ids: &[SubscriptionId],
        id_map: &HashMap<SubscriptionId, BitgetSubscription>,
    ) -> Vec<WsMessage> {
        // Implementation is the same as spot for Bitget
        Self::spot_subscription_messages(subscription_ids, id_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spot_subscribe_message() {
        let subscription =
            BitgetSubscription::new(BitgetChannel::ORDER_BOOK_L2, "BTCUSDT".to_string());
        let message = subscription.spot_subscribe_message();

        if let WsMessage::Text(text) = message {
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(json["op"], "subscribe");
            assert_eq!(json["args"][0]["channel"], "depth");
            assert_eq!(json["args"][0]["instId"], "BTCUSDT");
        } else {
            panic!("Expected Text message");
        }
    }

    #[test]
    fn test_futures_subscribe_message() {
        let subscription =
            BitgetSubscription::new(BitgetChannel::ORDER_BOOK_L2, "BTCUSDT_UMCBL".to_string());
        let message = subscription.futures_subscribe_message();

        if let WsMessage::Text(text) = message {
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(json["op"], "subscribe");
            assert_eq!(json["args"][0]["channel"], "depth");
            assert_eq!(json["args"][0]["instId"], "BTCUSDT_UMCBL");
        } else {
            panic!("Expected Text message");
        }
    }

    #[test]
    fn test_spot_unsubscribe_message() {
        let subscription =
            BitgetSubscription::new(BitgetChannel::ORDER_BOOK_L2, "BTCUSDT".to_string());
        let message = subscription.spot_unsubscribe_message();

        if let WsMessage::Text(text) = message {
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(json["op"], "unsubscribe");
            assert_eq!(json["args"][0]["channel"], "depth");
            assert_eq!(json["args"][0]["instId"], "BTCUSDT");
        } else {
            panic!("Expected Text message");
        }
    }

    #[test]
    fn test_spot_subscription_messages_batch() {
        let mut id_map = HashMap::new();
        let sub1 = BitgetSubscription::new(BitgetChannel::ORDER_BOOK_L2, "BTCUSDT".to_string());
        let sub2 = BitgetSubscription::new(BitgetChannel::ORDER_BOOK_L2, "ETHUSDT".to_string());

        let id1 = SubscriptionId::from("bitget:depth:BTCUSDT");
        let id2 = SubscriptionId::from("bitget:depth:ETHUSDT");

        id_map.insert(id1.clone(), sub1);
        id_map.insert(id2.clone(), sub2);

        let messages = BitgetSubscription::spot_subscription_messages(&[id1, id2], &id_map);

        assert_eq!(messages.len(), 1); // Should be batched into one message
        if let WsMessage::Text(text) = &messages[0] {
            let json: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(json["op"], "subscribe");
            assert_eq!(json["args"].as_array().unwrap().len(), 2);
            assert_eq!(json["args"][0]["channel"], "depth");
            assert!(
                json["args"][0]["instId"] == "BTCUSDT" || json["args"][0]["instId"] == "ETHUSDT"
            );
            assert!(
                json["args"][1]["instId"] == "BTCUSDT" || json["args"][1]["instId"] == "ETHUSDT"
            );
        } else {
            panic!("Expected Text message");
        }
    }
}

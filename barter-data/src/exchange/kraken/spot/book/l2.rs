use crate::{
    Identifier,
    books::{Level, OrderBook},
    exchange::kraken::message::KrakenMessage,
    subscription::book::OrderBookEvent,
    event::{MarketEvent, MarketIter},
};
use barter_integration::{
    de::extract_next,
    subscription::SubscriptionId,
};
use barter_instrument::exchange::ExchangeId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Terse type alias for an [`Kraken`](crate::exchange::kraken::KrakenSpot) real-time OrderBook Level2
/// (full depth) WebSocket message.
pub type KrakenOrderBookL2 = KrakenMessage<KrakenOrderBookL2Inner>;

/// [`Kraken`](crate::exchange::kraken::KrakenSpot) L2 OrderBook data and the
/// associated [`SubscriptionId`].
///
/// See docs: <https://docs.kraken.com/websockets/#message-book>
#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct KrakenOrderBookL2Inner {
    pub subscription_id: SubscriptionId,
    pub data: KrakenOrderBookL2Data,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum KrakenOrderBookL2Data {
    Snapshot(KrakenBookSnapshot),
    Update(KrakenBookUpdate),
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct KrakenBookSnapshot {
    #[serde(alias = "as")]
    pub asks: Vec<KrakenBookLevel>,
    #[serde(alias = "bs")]
    pub bids: Vec<KrakenBookLevel>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct KrakenBookUpdate {
    #[serde(default, rename = "a")]
    pub asks: Vec<KrakenBookLevel>,
    #[serde(default, rename = "b")]
    pub bids: Vec<KrakenBookLevel>,
    #[serde(rename = "c")]
    pub checksum: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub struct KrakenBookLevel {
    pub price: Decimal,
    pub amount: Decimal,
}

impl From<KrakenBookLevel> for Level {
    fn from(level: KrakenBookLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

impl Identifier<Option<SubscriptionId>> for KrakenOrderBookL2Inner {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl<'de> Deserialize<'de> for KrakenOrderBookL2Inner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = KrakenOrderBookL2Inner;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenOrderBookL2Inner struct from the Kraken WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // Kraken OrderBook L2 Format:
                // [channelID, data, channelName, pair]
                // <https://docs.kraken.com/websockets/#message-book>

                // Extract deprecated channelID & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelID")?;

                // Extract Data (Snapshot or Update)
                let data = extract_next(&mut seq, "KrakenOrderBookL2Data")?;

                // Extract channelName (eg/ "book-100") & ignore
                let _: serde::de::IgnoredAny = extract_next(&mut seq, "channelName")?;

                // Extract pair (eg/ "XBT/USD") & map to SubscriptionId (ie/ "book|{pair}")
                let subscription_id = extract_next::<SeqAccessor, String>(&mut seq, "pair")
                    .map(|pair| SubscriptionId::from(format!("book|{pair}")))?;

                // Ignore any additional elements
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenOrderBookL2Inner {
                    subscription_id,
                    data,
                })
            }
        }

        deserializer.deserialize_seq(SeqVisitor)
    }
}

impl<'de> Deserialize<'de> for KrakenBookLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LevelVisitor;

        impl<'de> serde::de::Visitor<'de> for LevelVisitor {
            type Value = KrakenBookLevel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("KrakenBookLevel array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let price = extract_next::<A, String>(&mut seq, "price")?
                    .parse()
                    .map_err(serde::de::Error::custom)?;
                let amount = extract_next::<A, String>(&mut seq, "amount")?
                    .parse()
                    .map_err(serde::de::Error::custom)?;
                
                // Consume remaining elements
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(KrakenBookLevel { price, amount })
            }
        }
        
        deserializer.deserialize_seq(LevelVisitor)
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, KrakenOrderBookL2)>
    for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from((exchange, instrument, message): (ExchangeId, InstrumentKey, KrakenOrderBookL2)) -> Self {
        let KrakenOrderBookL2Inner { data, .. } = match message {
             KrakenMessage::Data(data) => data,
             KrakenMessage::Event(_) => return Self(vec![]),
        };

        let event = match data {
            KrakenOrderBookL2Data::Snapshot(snap) => {
                OrderBookEvent::Snapshot(OrderBook::new(
                     0,
                     Some(Utc::now()),
                     snap.bids.into_iter().map(Level::from),
                     snap.asks.into_iter().map(Level::from),
                ))
            },
            KrakenOrderBookL2Data::Update(update) => {
                OrderBookEvent::Update(OrderBook::new(
                     0,
                     Some(Utc::now()),
                     update.bids.into_iter().map(Level::from),
                     update.asks.into_iter().map(Level::from),
                ))
            }
        };

        Self(vec![Ok(MarketEvent {
            exchange,
            instrument,
            kind: event,
            time_exchange: Utc::now(),
            time_received: Utc::now(),
        })])
    }
}

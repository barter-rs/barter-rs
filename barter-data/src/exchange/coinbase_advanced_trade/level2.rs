use crate::books::OrderBook;
use barter_instrument::exchange::ExchangeId;
use barter_integration::subscription::SubscriptionId;
use barter_integration::Side;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::coinbase_advanced_trade::channel::CoinbaseInternationalChannel;
use crate::exchange::coinbase_advanced_trade::message::CoinbaseInternationalMessage;
use crate::exchange::subscription::ExchangeSub;
use crate::subscription::book::OrderBookEvent;
use crate::Identifier;

/// Coinbase level2 WebSocket message.
/// ### Raw Payload Examples

/// ```json
/// {
///   "channel": "l2_data",
///   "client_id": "",
///   "timestamp": "2023-02-09T20:32:50.714964855Z",
///   "sequence_num": 0,
///   "events": [
///     {
///       "type": "snapshot",
///       "product_id": "BTC-USD",
///       "updates": [
///         {
///           "side": "bid",
///           "event_time": "1970-01-01T00:00:00Z",
///           "price_level": "21921.73",
///           "new_quantity": "0.06317902"
///         },
///         {
///           "side": "bid",
///           "event_time": "1970-01-01T00:00:00Z",
///           "price_level": "21921.3",
///           "new_quantity": "0.02"
///         }
///       ]
///     }
///   ]
/// }
/// ```
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Level2Event {
    #[serde(rename = "snapshot")]
    Snapshot(Level2),
    #[serde(rename = "update")]
    Update(Level2),
}
impl Level2Event {
    pub fn is_snapshot(&self) -> bool {
        matches!(self, Level2Event::Snapshot(_))
    }
    pub fn data(&self) -> &Level2 {
        match self {
            Level2Event::Snapshot(data) => data,
            Level2Event::Update(data) => data,
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Level2 {
    pub product_id: String,
    pub updates: Vec<Level2Update>,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Level2Update {
    pub side: Side,
    pub event_time: DateTime<Utc>,
    #[serde(with = "rust_decimal::serde::str")]
    pub price_level: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub new_quantity: Decimal,
}

impl Identifier<Option<SubscriptionId>> for CoinbaseInternationalMessage<Level2Event> {
    fn id(&self) -> Option<SubscriptionId> {
        match self.events.first() {
            None => None,
            Some(first_event) => {
                let data = first_event.data();
                let product_id = data.product_id.as_str();
                ExchangeSub::from((CoinbaseInternationalChannel::LEVEL2, product_id))
                    .id()
                    .into()
            }
        }
    }
}

impl<InstrumentKey>
    From<(
        ExchangeId,
        InstrumentKey,
        CoinbaseInternationalMessage<Level2Event>,
    )> for MarketIter<InstrumentKey, OrderBookEvent>
where
    InstrumentKey: Clone,
{
    fn from(
        (exchange_id, instrument, message): (
            ExchangeId,
            InstrumentKey,
            CoinbaseInternationalMessage<Level2Event>,
        ),
    ) -> Self {
        let events: Vec<_> = message
            .events
            .iter()
            .map(|event| {
                let data = event.data();
                // all updates have the same event_time
                let event_time = data.updates.first().map(|u| u.event_time).unwrap_or(message.timestamp);
                let order_book = OrderBook::new(
                    message.sequence_num,
                    Some(event_time),
                    data.updates
                        .iter()
                        .filter(|u| u.side == Side::Buy)
                        .map(move |update| (update.price_level, update.new_quantity)),
                    data.updates
                        .iter()
                        .filter(|u| u.side == Side::Sell)
                        .map(move |update| (update.price_level, update.new_quantity)),
                );
                Ok(MarketEvent {
                    time_exchange: message.timestamp,
                    time_received: Utc::now(),
                    exchange: exchange_id,
                    instrument: instrument.clone(),
                    kind: if event.is_snapshot() {
                        OrderBookEvent::Snapshot(order_book)
                    } else {
                        OrderBookEvent::Update(order_book)
                    },
                })
            })
            .collect::<Vec<_>>();
        Self(events)
    }
}

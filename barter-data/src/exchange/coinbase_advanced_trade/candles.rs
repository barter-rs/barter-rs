use crate::event::{MarketEvent, MarketIter};
use crate::exchange::coinbase_advanced_trade::channel::CoinbaseInternationalChannel;
use crate::exchange::coinbase_advanced_trade::message::CoinbaseInternationalMessage;
use crate::exchange::subscription::ExchangeSub;
use crate::subscription::candle::{Candle, Candles};
use crate::Identifier;
use barter_instrument::exchange::ExchangeId;
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

/// Coinbase candles WebSocket message.
/// ### Raw Payload Examples
/// ```json
/// {
///   "channel": "candles",
///   "client_id": "",
///   "timestamp": "2023-06-09T20:19:35.39625135Z",
///   "sequence_num": 0,
///   "events": [
///     {
///       "type": "snapshot",
///       "candles": [
///         {
///           "start": "1688998200",
///           "high": "1867.72",
///           "low": "1865.63",
///           "open": "1867.38",
///           "close": "1866.81",
///           "volume": "0.20269406",
///           "product_id": "ETH-USD"
///         }
///       ]
///     }
///   ]
/// }
/// ```

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content  = "candles")]
pub enum CandleEvent {
    #[serde(rename = "snapshot")]
    Snapshot(Vec<CBCandle>),
    #[serde(rename = "update")]
    Update(Vec<CBCandle>),
}

impl CandleEvent {
    pub fn data(&self) -> &[CBCandle] {
        match self {
            CandleEvent::Snapshot(data) => data,
            CandleEvent::Update(data) => data,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
struct CBCandle {
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    start: u64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    high: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    low: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]

    open: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    close: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    volume: f64,
    product_id: String,
}

impl Identifier<Option<SubscriptionId>> for CoinbaseInternationalMessage<CandleEvent> {
    fn id(&self) -> Option<SubscriptionId> {
        match self.events.first() {
            None => None,
            Some(first_event) => match first_event.data().first() {
                None => None,
                Some(candle) => {
                    let product_id = candle.product_id.as_str();
                    ExchangeSub::from((CoinbaseInternationalChannel::CANDLES, product_id))
                        .id()
                        .into()
                }
            },
        }
    }
}

impl<InstrumentKey>
    From<(
        ExchangeId,
        InstrumentKey,
        CoinbaseInternationalMessage<CandleEvent>,
    )> for MarketIter<InstrumentKey, Candle>
where
    InstrumentKey: Clone,
{
    fn from(
        (exchange_id, instrument, message): (
            ExchangeId,
            InstrumentKey,
            CoinbaseInternationalMessage<CandleEvent>,
        ),
    ) -> Self {
        let events: Vec<_> = message
            .events
            .iter()
            .flat_map(|event| {
                let data = event.data();
                data.iter().map(|c| {
                    Ok(MarketEvent {
                        time_exchange: message.timestamp,
                        time_received: Utc::now(),
                        exchange: exchange_id,
                        instrument: instrument.clone(),
                        kind: Candle {
                            // FIXME: check coinbase candle's start with barter close_time
                            close_time: DateTime::<Utc>::from_timestamp(c.start as i64, 0).unwrap(),
                            open: c.open,
                            high: c.high,
                            low: c.low,
                            close: c.close,
                            volume: c.volume,
                            trade_count: 0,
                        },
                    })
                })
            })
            .collect::<Vec<_>>()
            .into();
        Self(events)
    }
}

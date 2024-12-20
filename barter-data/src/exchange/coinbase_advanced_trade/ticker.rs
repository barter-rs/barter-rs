use crate::books::Level;
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::coinbase_advanced_trade::message::CoinbaseInternationalMessage;
use crate::exchange::subscription::ExchangeSub;
use crate::subscription::book::OrderBookL1;
use crate::Identifier;
use barter_instrument::exchange::ExchangeId;
use barter_integration::subscription::SubscriptionId;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::exchange::coinbase_advanced_trade::channel::CoinbaseInternationalChannel;

/// Coinbase ticker WebSocket message.
/// ### Raw Payload Examples

/// ```json
/// {
///   "channel": "ticker",
///   "client_id": "",
///   "timestamp": "2023-02-09T20:30:37.167359596Z",
///   "sequence_num": 0,
///   "events": [
///     {
///       "type": "snapshot",
///       "tickers": [
///         {
///           "type": "ticker",
///           "product_id": "BTC-USD",
///           "price": "21932.98",
///           "volume_24_h": "16038.28770938",
///           "low_24_h": "21835.29",
///           "high_24_h": "23011.18",
///           "low_52_w": "15460",
///           "high_52_w": "48240",
///           "price_percent_chg_24_h": "-4.15775596190603",
///           "best_bid": "21931.98",
///           "best_bid_quantity": "8000.21",
///           "best_ask": "21933.98",
///           "best_ask_quantity": "8038.07770938"
///         }
///       ]
///     }
///   ]
/// }
/// ```

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "tickers")]
pub enum TickerEvent {
    #[serde(rename = "snapshot")]
    Snapshot(Vec<Ticker>),
    #[serde(rename = "update")]
    Update(Vec<Ticker>),
}
impl TickerEvent {
    pub fn tickers(&self) -> &[Ticker] {
        match self {
            TickerEvent::Snapshot(tickers) => tickers,
            TickerEvent::Update(tickers) => tickers,
        }
    }
}
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Ticker {
    pub product_id: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub volume_24_h: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub low_24_h: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub high_24_h: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub low_52_w: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub high_52_w: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub best_bid: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub best_bid_quantity: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub best_ask: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub best_ask_quantity: Decimal,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price_percent_chg_24_h: f64,
}

impl Identifier<Option<SubscriptionId>> for CoinbaseInternationalMessage<TickerEvent> {
    fn id(&self) -> Option<SubscriptionId> {
        match self.events.first() {
            None => None,
            Some(first_event) => {
                let ticker = first_event.tickers().first();
                match ticker {
                    None => None,
                    Some(ticker) => {
                        let product_id = ticker.product_id.as_str();
                        ExchangeSub::from((CoinbaseInternationalChannel::TICKER.as_ref(), product_id))
                            .id()
                            .into()
                    }
                }
            }
        }
    }
}

impl<InstrumentKey>
    From<(
        ExchangeId,
        InstrumentKey,
        CoinbaseInternationalMessage<TickerEvent>,
    )> for MarketIter<InstrumentKey, OrderBookL1>
where
    InstrumentKey: Clone,
{
    fn from(
        (exchange_id, instrument, message): (
            ExchangeId,
            InstrumentKey,
            CoinbaseInternationalMessage<TickerEvent>,
        ),
    ) -> Self {
        let events: Vec<_> = message
            .events
            .iter()
            .flat_map(|event| {
                event.tickers().iter().map(|ticker| {
                    Ok(MarketEvent {
                        time_exchange: message.timestamp,
                        time_received: Utc::now(),
                        exchange: exchange_id,
                        instrument: instrument.clone(),
                        kind: OrderBookL1 {
                            last_update_time: message.timestamp,
                            best_bid: Level::new(ticker.best_bid, ticker.best_bid_quantity),
                            best_ask: Level {
                                price: ticker.best_ask,
                                amount: ticker.best_ask_quantity,
                            },
                        },
                    })
                })
            })
            .collect::<Vec<_>>();
        Self(events)
    }
}

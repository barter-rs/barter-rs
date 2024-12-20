use crate::exchange::coinbase_advanced_trade::channel::CoinbaseInternationalChannel;
use crate::{
    event::{MarketEvent, MarketIter},
    exchange::ExchangeSub,
    subscription::trade::PublicTrade,
    Identifier,
};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{subscription::SubscriptionId, Side};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::exchange::coinbase_advanced_trade::message::CoinbaseInternationalMessage;

/// Coinbase real-time trade WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://docs.cdp.coinbase.com/advanced-trade/docs/ws-channels#market-trades-channel>
/// ```json
///   "client_id": "",
///   "timestamp": "2023-02-09T20:19:35.39625135Z",
///   "sequence_num": 0,
///   "events": [
///     {
///       "type": "snapshot",
///       "trades": [
///         {
///           "trade_id": "000000000",
///           "product_id": "ETH-USD",
///           "price": "1260.01",
///           "size": "0.3",
///           "side": "BUY",
///           "time": "2019-08-14T20:42:27.265Z"
///         }
///       ]
///     }
///   ]
/// }
/// ```
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "trades")]
pub enum MarketTradeEvent {
    #[serde(rename = "snapshot")]
    Snapshot(Vec<Trade>),
    #[serde(rename = "update")]
    Update(Vec<Trade>),
}
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct Trade {
    trade_id: String,
    product_id: String,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    price: f64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    size: f64,
    side: Side,
    time: DateTime<Utc>,
}

impl Identifier<Option<SubscriptionId>> for CoinbaseInternationalMessage<MarketTradeEvent> {
    fn id(&self) -> Option<SubscriptionId> {
        match self.events.first() {
            None => None,
            Some(first_event) => {
                let trade = match first_event {
                    MarketTradeEvent::Snapshot(trades) => trades,
                    MarketTradeEvent::Update(trades) => trades,
                }.first();
                match trade {
                    None => {
                        None
                    }
                    Some(trade) => {
                        let product_id = trade.product_id.as_str();
                        ExchangeSub::from((self.channel.as_str(), product_id))
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
        CoinbaseInternationalMessage<MarketTradeEvent>,
    )> for MarketIter<InstrumentKey, PublicTrade>
where InstrumentKey: Clone {
    fn from(
        (exchange_id, instrument, trade): (
            ExchangeId,
            InstrumentKey,
            CoinbaseInternationalMessage<MarketTradeEvent>,
        ),
    ) -> Self {
        let events: Vec<_> = trade
            .events
            .iter()
            .flat_map(|event| match event {
                MarketTradeEvent::Snapshot(snapshot) => snapshot
                    .iter()
                    .map(|trade| {
                        Ok(MarketEvent {
                            time_exchange: trade.time,
                            time_received: Utc::now(),
                            exchange: exchange_id,
                            instrument: instrument.clone(),
                            kind: PublicTrade {
                                id: trade.trade_id.clone(),
                                price: trade.price,
                                amount: trade.size,
                                side: trade.side,
                            },
                        })
                    })
                    .collect::<Vec<_>>(),
                MarketTradeEvent::Update(update) => update
                    .iter()
                    .map(|trade| {
                        Ok(MarketEvent {
                            time_exchange: trade.time,
                            time_received: Utc::now(),
                            exchange: exchange_id,
                            instrument: instrument.clone(),
                            kind: PublicTrade {
                                id: trade.trade_id.clone(),
                                price: trade.price,
                                amount: trade.size,
                                side: trade.side,
                            },
                        })
                    })
                    .collect::<Vec<_>>(),
            })
            .collect::<Vec<_>>();
        Self(events)
    }
}

/// Deserialize a [`MarketTradeEvent`] "product_id" (eg/ "BTC-USD") as the associated [`SubscriptionId`]
/// (eg/ SubscriptionId("matches|BTC-USD").
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer).map(|product_id| {
        ExchangeSub::from((CoinbaseInternationalChannel::MARKET_TRADES, product_id)).id()
    })
}

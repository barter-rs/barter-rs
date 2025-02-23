use super::super::message::GateioMessage;
use crate::{
    Identifier,
    event::{MarketEvent, MarketIter},
    exchange::ExchangeSub,
    subscription::trade::PublicTrade,
};
use barter_instrument::{Side, exchange::ExchangeId};
use barter_integration::subscription::SubscriptionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Terse type alias for an [`GateioSpot`](super::GateioSpot) real-time trades WebSocket message.
pub type GateioSpotTrade = GateioMessage<GateioSpotTradeInner>;

/// [`GateioSpot`](super::GateioSpot) real-time trade WebSocket message.
///
/// ### Raw Payload Examples
/// See docs: <https://www.gate.io/docs/developers/apiv4/ws/en/#public-trades-channel>
/// ```json
/// {
///   "id": 309143071,
///   "create_time": 1606292218,
///   "create_time_ms": "1606292218213.4578",
///   "side": "sell",
///   "currency_pair": "GT_USDT",
///   "amount": "16.4700000000",
///   "price": "0.4705000000"
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct GateioSpotTradeInner {
    #[serde(rename = "currency_pair")]
    pub market: String,
    #[serde(
        rename = "create_time_ms",
        deserialize_with = "barter_integration::de::de_str_f64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    pub id: u64,
    #[serde(deserialize_with = "barter_integration::de::de_str")]
    pub price: f64,

    #[serde(alias = "size", deserialize_with = "barter_integration::de::de_str")]
    pub amount: f64,

    /// Taker [`Side`] of the trade.
    pub side: Side,
}

impl Identifier<Option<SubscriptionId>> for GateioSpotTrade {
    fn id(&self) -> Option<SubscriptionId> {
        Some(ExchangeSub::from((&self.channel, &self.data.market)).id())
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, GateioSpotTrade)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from(
        (exchange_id, instrument, trade): (ExchangeId, InstrumentKey, GateioSpotTrade),
    ) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: trade.data.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: PublicTrade {
                id: trade.data.id.to_string(),
                price: trade.data.price,
                amount: trade.data.amount,
                side: trade.data.side,
            },
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_gateio_message_futures_trade() {
            let input = r#"
            {
                "time": 1606292218,
                "time_ms": 1606292218231,
                "channel": "spot.trades",
                "event": "update",
                "result": {
                    "id": 309143071,
                    "create_time": 1606292218,
                    "create_time_ms": "1606292218213.4578",
                    "side": "sell",
                    "currency_pair": "GT_USDT",
                    "amount": "16.4700000000",
                    "price": "0.4705000000"
                }
            }
            "#;
            serde_json::from_str::<GateioSpotTrade>(input).unwrap();
        }
    }
}

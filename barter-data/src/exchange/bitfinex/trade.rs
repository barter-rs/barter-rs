use crate::{
    event::{MarketEvent, MarketIter},
    subscription::trade::PublicTrade,
};
use barter_instrument::{Side, exchange::ExchangeId};
use barter_integration::de::{datetime_utc_from_epoch_duration, extract_next};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// [`Bitfinex`](super::Bitfinex) real-time trade message.
///
/// ### Raw Payload Examples
/// Format: \[ID, TIME, AMOUNT, PRICE\], <br> where +/- of amount indicates Side
///
/// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
///
/// #### Side::Buy Trade
/// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
/// ```json
/// [420191,"te",[1225484398,1665452200022,0.08980641,19027.02807752]]
/// ```
///
/// #### Side::Sell Trade
/// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
/// ```json
/// [420191,"te",[1225484398,1665452200022,-0.08980641,19027.02807752]]
/// ```
///
/// ## Notes:
/// - [`Bitfinex`](super::Bitfinex) trades subscriptions results in receiving tag="te" & tag="tu"
///   trades, both of which are identical.
/// - "te" trades arrive marginally faster.
/// - Therefore, tag="tu" trades are filtered out and considered only as additional Heartbeats.
///
/// See docs: <https://docs.bitfinex.com/reference/ws-public-trades>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
pub struct BitfinexTrade {
    pub id: u64,
    pub time: DateTime<Utc>,
    pub side: Side,
    pub price: f64,
    pub amount: f64,
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BitfinexTrade)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from((exchange_id, instrument, trade): (ExchangeId, InstrumentKey, BitfinexTrade)) -> Self {
        Self(vec![Ok(MarketEvent {
            time_exchange: trade.time,
            time_received: Utc::now(),
            exchange: exchange_id,
            instrument,
            kind: PublicTrade {
                id: trade.id.to_string(),
                price: trade.price,
                amount: trade.amount,
                side: trade.side,
            },
        })])
    }
}

impl<'de> serde::Deserialize<'de> for BitfinexTrade {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = BitfinexTrade;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("BitfinexTrade struct from the Bitfinex WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // Trade: [ID, TIME, AMOUNT,PRICE]
                let id = extract_next(&mut seq, "id")?;
                let time_millis = extract_next(&mut seq, "time")?;
                let amount: f64 = extract_next(&mut seq, "amount")?;
                let price = extract_next(&mut seq, "price")?;
                let side = match amount.is_sign_positive() {
                    true => Side::Buy,
                    false => Side::Sell,
                };

                // Ignore any additional elements or SerDe will fail
                //  '--> Bitfinex may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}

                Ok(BitfinexTrade {
                    id,
                    time: datetime_utc_from_epoch_duration(std::time::Duration::from_millis(
                        time_millis,
                    )),
                    price,
                    amount: amount.abs(),
                    side,
                })
            }
        }

        // Use Visitor implementation to deserialise the BitfinexTrade message
        deserializer.deserialize_seq(SeqVisitor)
    }
}

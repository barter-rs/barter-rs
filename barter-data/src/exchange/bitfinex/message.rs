use super::trade::BitfinexTrade;
use crate::{Identifier, event::MarketIter, subscription::trade::PublicTrade};
use barter_instrument::exchange::ExchangeId;
use barter_integration::{de::extract_next, subscription::SubscriptionId};
use serde::Serialize;

/// [`Bitfinex`](super::Bitfinex) message received over
/// [`WebSocket`](barter_integration::protocol::websocket::WebSocket) relating to an active
/// [`Subscription`](crate::Subscription).
///
/// The message is associated with the original [`Subscription`](crate::Subscription) using the
/// `channel_id` field as the [`SubscriptionId`].
///
/// ### Raw Payload Examples
/// #### Heartbeat
/// See docs: <https://docs.bitfinex.com/docs/ws-general#heartbeating>
/// ```json
/// [420191,"hb"]
/// ```
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
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
pub struct BitfinexMessage {
    pub channel_id: u32,
    pub payload: BitfinexPayload,
}

/// [`Bitfinex`](super::Bitfinex) market data variants associated with an
/// active [`Subscription`](crate::Subscription).
///
/// See [`BitfinexMessage`] for full raw payload examples.
///
/// See docs: <https://docs.bitfinex.com/docs/ws-general>
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize)]
pub enum BitfinexPayload {
    Heartbeat,
    Trade(BitfinexTrade),
}

impl Identifier<Option<SubscriptionId>> for BitfinexMessage {
    fn id(&self) -> Option<SubscriptionId> {
        match self.payload {
            BitfinexPayload::Heartbeat => None,
            BitfinexPayload::Trade(_) => Some(SubscriptionId::from(self.channel_id.to_string())),
        }
    }
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, BitfinexMessage)>
    for MarketIter<InstrumentKey, PublicTrade>
{
    fn from(
        (exchange_id, instrument, message): (ExchangeId, InstrumentKey, BitfinexMessage),
    ) -> Self {
        match message.payload {
            BitfinexPayload::Heartbeat => Self(vec![]),
            BitfinexPayload::Trade(trade) => Self::from((exchange_id, instrument, trade)),
        }
    }
}

impl<'de> serde::Deserialize<'de> for BitfinexMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct SeqVisitor;

        impl<'de> serde::de::Visitor<'de> for SeqVisitor {
            type Value = BitfinexMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("BitfinexMessage struct from the Bitfinex WebSocket API")
            }

            fn visit_seq<SeqAccessor>(
                self,
                mut seq: SeqAccessor,
            ) -> Result<Self::Value, SeqAccessor::Error>
            where
                SeqAccessor: serde::de::SeqAccess<'de>,
            {
                // Trade: [CHANNEL_ID, <"te", "tu">, [ID, TIME, AMOUNT, PRICE]]
                // Heartbeat: [ CHANNEL_ID, "hb" ]
                // Candle: [CHANNEL_ID, [MTS, OPEN, CLOSE, HIGH, LOW, VOLUME]]

                // Extract CHANNEL_ID used to identify SubscriptionId: 1st element of the sequence
                let channel_id: u32 = extract_next(&mut seq, "channel_id")?;

                // Extract message tag to identify payload type: 2nd element of the sequence
                let message_tag: String = extract_next(&mut seq, "message_tag")?;

                // Use message tag to extract the payload: 3rd element of sequence
                let payload = match message_tag.as_str() {
                    // Filter "tu" Trades since they are identical but slower
                    // '--> use as additional Heartbeat
                    "hb" | "tu" => BitfinexPayload::Heartbeat,
                    "te" => BitfinexPayload::Trade(extract_next(&mut seq, "BitfinexTrade")?),
                    other => {
                        return Err(serde::de::Error::unknown_variant(
                            other,
                            &["heartbeat (hb)", "trade (te | tu)"],
                        ));
                    }
                };

                // Ignore any additional elements or SerDe will fail
                //  '--> Bitfinex may add fields without warning
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
                Ok(BitfinexMessage {
                    channel_id,
                    payload,
                })
            }
        }

        // Use Visitor implementation to deserialise the WebSocket BitfinexMessage
        deserializer.deserialize_seq(SeqVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use barter_instrument::Side;
    use barter_integration::{de::datetime_utc_from_epoch_duration, error::SocketError};
    use std::time::Duration;

    #[test]
    fn test_de_bitfinex_message() {
        struct TestCase {
            input: &'static str,
            expected: Result<BitfinexMessage, SocketError>,
        }

        // Trade: [CHANNEL_ID, <"te", "tu">, [ID, TIME, AMOUNT, PRICE]]
        // Heartbeat: [ CHANNEL_ID, "hb" ]
        // Candle: [CHANNEL_ID, [MTS, OPEN, CLOSE, HIGH, LOW, VOLUME]]

        let cases = vec![
            // TC0: Trade message te Sell
            TestCase {
                input: r#"[420191,"te",[1225484398,1665452200022,-0.08980641,19027.02807752]]"#,
                expected: Ok(BitfinexMessage {
                    channel_id: 420191,
                    payload: BitfinexPayload::Trade(BitfinexTrade {
                        id: 1225484398,
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1665452200022,
                        )),
                        side: Side::Sell,
                        price: 19027.02807752,
                        amount: 0.08980641,
                    }),
                }),
            },
            // TC1: Trade message te Buy
            TestCase {
                input: r#"[420191,"te",[1225484398,1665452200022,0.08980641,19027.02807752]]"#,
                expected: Ok(BitfinexMessage {
                    channel_id: 420191,
                    payload: BitfinexPayload::Trade(BitfinexTrade {
                        id: 1225484398,
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1665452200022,
                        )),
                        side: Side::Buy,
                        price: 19027.02807752,
                        amount: 0.08980641,
                    }),
                }),
            },
            // TC2: Trade tu --> Should be marked as a heartbeat
            TestCase {
                input: r#"[420191,"tu",[1225484398,1665452200022,-0.08980641,19027.02807752]]"#,
                expected: Ok(BitfinexMessage {
                    channel_id: 420191,
                    payload: BitfinexPayload::Heartbeat,
                }),
            },
            // TC3: Heartbeat message
            TestCase {
                input: r#"[420191,"hb"]"#,
                expected: Ok(BitfinexMessage {
                    channel_id: 420191,
                    payload: BitfinexPayload::Heartbeat,
                }),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<BitfinexMessage>(test.input);
            match (actual, test.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "TC{} failed", index)
                }
                (Err(_), Err(_)) => {
                    // Test passed
                }
                (actual, expected) => {
                    // Test failed
                    panic!(
                        "TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n"
                    );
                }
            }
        }
    }
}

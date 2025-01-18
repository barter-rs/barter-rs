use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::Side;
use barter_integration::protocol::websocket::WsMessage;
use barter_integration::subscription::SubscriptionId;
use barter_integration::Transformer;
use crate::books::{Level, OrderBook};
use crate::error::DataError;
use crate::event::{MarketEvent, MarketIter};
use crate::exchange::coinbase::Coinbase;
use crate::exchange::Connector;
use crate::Identifier;
use crate::subscription::book::{OrderBookEvent, OrderBooksL2};
use crate::subscription::Map;
use crate::transformer::ExchangeTransformer;

pub const HTTP_PRODUCT_BOOK_SNAPSHOT_URL: &str = "https://api.exchange.coinbase.com/products/{product_id}/book";

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseOrderBookSnapshot {
    pub sequence: f64,
    pub bids: Vec<CoinbaseOrderBookLevel>,
    pub asks: Vec<CoinbaseOrderBookLevel>,
    #[serde(
        alias = "time",
        deserialize_with = "barter_integration::de::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time_exchange: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinbaseOrderBookLevel {
    pub price: String,
    pub size: String,
    pub num_orders: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct CoinbaseOrderBookL2Update {
    #[serde(
        alias = "product_id",
    )]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "time"
    )]
    pub time_exchange: DateTime<Utc>,
    pub changes: Vec<CoinbaseOrderBookL2Change>,
}

impl Identifier<Option<SubscriptionId>> for CoinbaseOrderBookL2Update {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinbaseOrderBookL2Change {
    pub side: Side,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub size: Decimal,
}

#[derive(Debug, Constructor)]
pub struct CoinbaseOrderBookL2Meta<InstrumentKey, Sequencer> {
    pub key: InstrumentKey,
    pub sequencer: Sequencer,
}

#[derive(Debug, Default)]
pub struct CoinbaseOrderBookL2Sequencer {
    pub updates_processed: u64,
    pub last_updated_at: DateTime<Utc>
}

impl CoinbaseOrderBookL2Sequencer {
    pub fn new(last_updated_at: DateTime<Utc>) -> Self {
        Self {
            updates_processed: 0,
            last_updated_at
        }
    }

    pub fn validate_sequence(&mut self, update: CoinbaseOrderBookL2Update) -> Result<Option<CoinbaseOrderBookL2Update>, DataError> {
        if update.time_exchange <= self.last_updated_at {
            return Ok(None);
        }

        self.updates_processed += 1;
        self.last_updated_at = update.time_exchange;

        Ok(Some(update))
    }
}

#[derive(Debug)]
pub struct CoinbaseOrderBooksL2Transformer<InstrumentKey> {
    instrument_map: Map<CoinbaseOrderBookL2Meta<InstrumentKey, CoinbaseOrderBookL2Sequencer>>
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<Coinbase, InstrumentKey, OrderBooksL2> for CoinbaseOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        initial_snapshots: &[MarketEvent<InstrumentKey, OrderBookEvent>],
        _: UnboundedSender<WsMessage>
    ) -> Result<Self, DataError> {
        let instrument_map = instrument_map.0
            .into_iter()
            .map(|(sub_id, instrument_key)| {
                let snapshot = initial_snapshots
                    .iter()
                    .find(|snapshot| snapshot.instrument == instrument_key)
                    .ok_or_else(|| DataError::InitialSnapshotMissing(sub_id.clone()))?;

                let OrderBookEvent::Snapshot(snapshot) = &snapshot.kind else {
                    return Err(DataError::InitialSnapshotInvalid(String::from(
                        "expected OrderBookEvent::Snapshot but found OrderBookEvent::Update",
                    )));
                };

                Ok(
                    (
                        sub_id,
                        CoinbaseOrderBookL2Meta {
                            key: instrument_key.clone(),
                            sequencer: CoinbaseOrderBookL2Sequencer::new(Utc::now())
                        }
                    )
                )
            })
            .collect::<Result<Map<_>, _>>()?;

        Ok(Self { instrument_map })
    }
}

impl<InstrumentKey> Transformer for CoinbaseOrderBooksL2Transformer<InstrumentKey>
where
    InstrumentKey: Clone + PartialEq + Send + Sync
{
    type Error = DataError;
    type Input = CoinbaseOrderBookL2Update;
    type Output = MarketEvent<InstrumentKey, OrderBookEvent>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        let instrument = match self.instrument_map.find_mut(&subscription_id) {
            Ok(instrument) => instrument,
            Err(unidentifiable) => return vec![Err(DataError::from(unidentifiable))],
        };

        let valid_update = match instrument.sequencer.validate_sequence(input) {
            Ok(Some(valid_update)) => valid_update,
            Ok(None) => return vec![],
            Err(error) => return vec![Err(error)],
        };

        MarketIter::<InstrumentKey, OrderBookEvent>::from((
            Coinbase::ID,
            instrument.key.clone(),
            valid_update,
        )).0
    }
}

impl From<CoinbaseOrderBookL2Change> for Level {
    fn from(change: CoinbaseOrderBookL2Change) -> Self {
        Self {
            price: change.price,
            amount: change.size,
        }
    }
}

impl FromIterator<CoinbaseOrderBookL2Change> for Vec<Level> {
    fn from_iter<T: IntoIterator<Item=CoinbaseOrderBookL2Change>>(iter: T) -> Self {
        iter.into_iter().map(Level::from).collect()
    }
}


impl<InstrumentKey> From<(ExchangeId, InstrumentKey, CoinbaseOrderBookL2Update)>
for MarketIter<InstrumentKey, OrderBookEvent>
{
    fn from(
        (exchange, instrument, update): (
            ExchangeId,
            InstrumentKey,
            CoinbaseOrderBookL2Update,
        ),
    ) -> Self {
        let (bids, asks): (Vec<_>, Vec<_>) = update.changes.into_iter().partition(|change| change.side == Side::Buy);

        Self(vec![Ok(MarketEvent {
            time_exchange: update.time_exchange,
            time_received: Utc::now(),
            exchange,
            instrument,
            kind: OrderBookEvent::Update(OrderBook::new(
                0,
                Some(update.time_exchange),
                bids,
                asks,
            )),
        })])
    }
}

fn get_book_snapshot_url(symbol: &str) -> String {
    format!("https://api.exchange.coinbase.com/products/{symbol}/book", symbol = symbol)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use rust_decimal::Decimal;
    use barter_instrument::Side;
    use barter_integration::subscription::SubscriptionId;
    use super::{get_book_snapshot_url, CoinbaseOrderBookL2Change, CoinbaseOrderBookL2Update};

    #[test]
    fn test_get_book_snapshot_url() {
        assert_eq!(get_book_snapshot_url("BTC-USD"), "https://api.exchange.coinbase.com/products/BTC-USD/book");
    }

    #[test]
    fn test_de_coinbase_order_book_l2_update() {
        let input = r#"
            {
                "product_id": "BTC-USD",
                "time": "2022-08-04T15:25:05.010758Z",
                "changes": [
                    ["buy", "10000.00", "0.01"],
                    ["sell", "10001.00", "0.01"]
                ]
            }
        "#;

        assert_eq!(serde_json::from_str::<CoinbaseOrderBookL2Update>(input).unwrap(), CoinbaseOrderBookL2Update {
            subscription_id: SubscriptionId::from("BTC-USD"),
            time_exchange: chrono::DateTime::from_str("2022-08-04T15:25:05.010758Z").unwrap(),
            changes: vec![
                CoinbaseOrderBookL2Change {
                    side: Side::Buy,
                    price: Decimal::from_str("10000.00").unwrap(),
                    size: Decimal::from_str("0.01").unwrap()
                },
                CoinbaseOrderBookL2Change {
                    side: Side::Sell,
                    price: Decimal::from_str("10001.00").unwrap(),
                    size: Decimal::from_str("0.01").unwrap()
                },
            ]
        });
    }
}
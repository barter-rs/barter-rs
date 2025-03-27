use chrono::{DateTime, Utc};
use databento::dbn::{Action, ErrorMsg, MboMsg, Mbp10Msg, Mbp1Msg, RecordRef, TradeMsg, UNDEF_PRICE};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::instrument::InstrumentIndex;
use barter_instrument::Side;
use crate::books::{Level, OrderBook};
use crate::error::DataError;
use crate::event::{DataKind, MarketEvent};
use crate::provider::databento::DatabentoSide;
use crate::subscription::book::{OrderBookAction, OrderBookEvent, OrderBookL1, OrderBookUpdate, OrderBooksL2};
use crate::subscription::trade::PublicTrade;

impl From<(MboMsg, OrderBookAction)> for OrderBookUpdate {
    fn from(value: (MboMsg, OrderBookAction)) -> Self {
        let (mbo, action) = value;
        let side = mbo.side().unwrap();
        let price = mbo.price_f64();

        OrderBookUpdate {
            order_id: Some(mbo.order_id.to_string()),
            price: Decimal::from_f64(price).unwrap(),
            amount: Decimal::from(mbo.size),
            side: Side::from(DatabentoSide::from(side)),
            sequence: mbo.sequence as u64,
            action,
        }
    }
}

impl From<(Mbp10Msg, OrderBookAction)> for OrderBookUpdate {
    fn from(value: (Mbp10Msg, OrderBookAction)) -> Self {
        let (mbo, action) = value;
        let side = mbo.side().unwrap();
        let price = mbo.price_f64();

        OrderBookUpdate {
            order_id: None,
            price: Decimal::from_f64(price).unwrap(),
            amount: Decimal::from(mbo.size),
            side: Side::from(DatabentoSide::from(side)),
            sequence: mbo.sequence as u64,
            action,
        }
    }
}

impl<InstrumentKey> From<(InstrumentKey, TradeMsg)> for MarketEvent<InstrumentKey, PublicTrade> {
    fn from((instrument, trade): (InstrumentKey, TradeMsg)) -> Self {
        let time_exchange = DateTime::from_timestamp_nanos(
            trade.ts_recv as i64).to_utc();
        let exchange = ExchangeId::Other;
        let side = Side::from(DatabentoSide::from(trade.side().unwrap()));

        MarketEvent {
            time_exchange,
            time_received: time_exchange.clone(),
            exchange,
            instrument,
            kind: PublicTrade {
                id: trade.sequence.to_string(),
                price: trade.price_f64(),
                amount: trade.size as f64,
                side,
            },
        }
    }
}

impl<InstrumentKey> TryFrom<(InstrumentKey, MboMsg)> for MarketEvent<InstrumentKey, OrderBookEvent> {
    type Error = DataError;

    fn try_from((instrument, mbo): (InstrumentKey, MboMsg)) -> Result<Self, Self::Error> {
        if let Err(e) = mbo.action() {
            return Err(DataError::from(e));
        }

        let action = match mbo.action() {
            Ok(Action::Add) => Some(OrderBookAction::Add),
            Ok(Action::Modify) => Some(OrderBookAction::Modify),
            Ok(Action::Cancel) => Some(OrderBookAction::Cancel),
            _ => None
        };

        if action.is_none() {
            return Err(DataError::Generic("Unsupported action".to_string()));
        }

        let time_exchange = DateTime::from_timestamp_nanos(mbo.ts_recv as i64).to_utc();
        let exchange = ExchangeId::Other;

        Ok(MarketEvent {
            time_exchange,
            time_received: time_exchange.clone(),
            exchange,
            instrument,
            kind: OrderBookEvent::IncrementalUpdate(OrderBookUpdate::from((mbo, action.unwrap()))),
        })
    }
}

impl<InstrumentKey> TryFrom<(InstrumentKey, Mbp10Msg)> for MarketEvent<InstrumentKey, OrderBookEvent> {
    type Error = DataError;

    fn try_from((instrument, mbp10): (InstrumentKey, Mbp10Msg)) -> Result<Self, Self::Error> {
        if let Err(e) = mbp10.action() {
            return Err(DataError::from(e));
        }

        let action = match mbp10.action() {
            Ok(Action::Add) => Some(OrderBookAction::Add),
            Ok(Action::Modify) => Some(OrderBookAction::Modify),
            Ok(Action::Cancel) => Some(OrderBookAction::Cancel),
            _ => {
                None
            }
        };

        if action.is_none() {
            return Err(DataError::Generic("Unsupported action".to_string()));
        }

        let time_exchange = DateTime::from_timestamp_nanos(mbp10.ts_recv as i64).to_utc();
        let exchange = ExchangeId::Other;

        let (bids, asks): (Vec<Option<Level>>, Vec<Option<Level>>) = mbp10.levels.iter()
            .map(|bap| {
                let bid_level = if bap.bid_px == UNDEF_PRICE {
                    None
                } else {
                    Some(Level::from((Decimal::from_f64(bap.bid_px_f64()).unwrap(), Decimal::from_f64(bap.bid_sz as f64).unwrap())))
                };

                let ask_level = if bap.ask_px == UNDEF_PRICE {
                    None
                } else {
                    Some(Level::from((Decimal::from_f64(bap.ask_px_f64()).unwrap(), Decimal::from_f64(bap.ask_sz as f64).unwrap())))
                };

            (bid_level, ask_level)
        }).unzip();

        Ok(MarketEvent {
            time_exchange,
            time_received: time_exchange.clone(),
            exchange,
            instrument,
            kind: OrderBookEvent::Update(
                OrderBook::new(
                    mbp10.sequence as u64,
                    Some(time_exchange),
                    bids.iter().filter(|l| l.is_some()).map(|l| l.unwrap()).collect::<Vec<Level>>(),
                    asks.iter().filter(|l| l.is_some()).map(|l| l.unwrap()).collect::<Vec<Level>>()
                )
            ),
        })

    }
}

impl<InstrumentKey> From<(InstrumentKey, Mbp1Msg)> for MarketEvent<InstrumentKey, OrderBookL1> {
    fn from((instrument, mbp1) : (InstrumentKey, Mbp1Msg)) -> Self {
        let tob = mbp1.levels.get(0).unwrap();

        let time_exchange = DateTime::from_timestamp_nanos(mbp1.ts_recv as i64).to_utc();
        let exchange = ExchangeId::Other;
        let kind = OrderBookL1 {
            last_update_time: time_exchange,
            best_bid: Decimal::from_f64(tob.bid_px_f64()).map(|price| Level {
                price,
                amount: Decimal::from(tob.bid_sz),
            }),
            best_ask: Decimal::from_f64(tob.ask_px_f64()).map(|price| Level {
                price,
                amount: Decimal::from(tob.ask_sz),
            }),
        };

        MarketEvent {
            time_exchange,
            time_received: time_exchange.clone(),
            exchange,
            instrument,
            kind,
        }
    }
}

pub fn transform_mbo(mbo: &MboMsg) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {
    if mbo.price == UNDEF_PRICE {
        return Ok(None);
    }

    let result = MarketEvent::try_from(
        (InstrumentIndex(0), mbo.clone()));

    Ok(Some(MarketEvent::from(result?)))
}

fn transform_mbp1(mbp1: &Mbp1Msg) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {
    Ok(Some(MarketEvent::from(MarketEvent::from((InstrumentIndex(0), mbp1.clone())))))
}

fn transform_mbp10(mbp10: &Mbp10Msg) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {
    let result = MarketEvent::try_from(
        (InstrumentIndex(0), mbp10.clone()));

    Ok(Some(MarketEvent::from(result?)))
}

pub fn transform(record_ref: RecordRef<'_>) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {

    if let Some(e) = record_ref.get::<ErrorMsg>() {
        return Err(DataError::from(e.clone()));
    }

    if let Some(trade) = record_ref.get::<TradeMsg>() {
        return transform_trade(trade);
    }

    if let Some(mbo) = record_ref.get::<MboMsg>() {
        return transform_mbo(mbo);
    }

    if let Some(mbp1) = record_ref.get::<Mbp1Msg>() {
        return transform_mbp1(mbp1);
    }

    if let Some(mbp10) = record_ref.get::<Mbp10Msg>() {
        return transform_mbp10(mbp10);
    }

    Ok(None)
}

fn transform_trade(p0: &TradeMsg) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {
    let trade = MarketEvent::from(
        (InstrumentIndex(0), p0.clone()));
    Ok(Some(MarketEvent::from(trade)))
}

#[cfg(test)]
mod tests {
    use std::ffi::c_char;
    use databento::dbn::{rtype, BidAskPair, FlagSet, RecordHeader, UNDEF_TIMESTAMP};
    use super::*;

    #[test]
    fn test_mbp1_to_orderbook_l1() {
        let mbp1 = Mbp1Msg {
            hd: RecordHeader::default::<Mbp1Msg>(rtype::MBP_1),
            price: 0,
            size: 0,
            action: 0,
            side: 0,
            flags: Default::default(),
            levels: [BidAskPair {
                bid_px: 100_000_000_000,
                ask_px: 101_000_000_000,
                bid_sz: 100,
                ask_sz: 100,
                bid_ct: 0,
                ask_ct: 0,
            }],
            ts_recv: UNDEF_TIMESTAMP,
            ts_in_delta: 0,
            sequence: 0,
            depth: 0,
        };
        let instrument = InstrumentIndex(0);
        let time = DateTime::from_timestamp_nanos(u64::MAX as i64).to_utc();

        struct TestCase {
            input: (InstrumentIndex, Mbp1Msg),
            expected: MarketEvent<InstrumentIndex, OrderBookL1>,
        }

        let test_cases = vec![
            TestCase {
                input: (instrument, mbp1.clone()),
                expected:  MarketEvent {
                    time_exchange: time,
                    time_received: time,
                    exchange: ExchangeId::Other,
                    instrument,
                    kind: OrderBookL1 {
                        last_update_time: time,
                        best_bid: Some(Level {
                            price: Decimal::from_f64(100.00).unwrap(),
                            amount: Decimal::from(100),
                        }),
                        best_ask: Some(Level {
                            price: Decimal::from_f64(101.00).unwrap(),
                            amount: Decimal::from(100),
                        }),
                    }
                },
            },
        ];

        for test_case in test_cases {
            let result = MarketEvent::from(test_case.input);
            assert_eq!(result, test_case.expected);
        }
    }

    #[test]
    fn test_mbo_to_orderbook_l3() {
        let mbo = MboMsg {
            hd: RecordHeader::default::<MboMsg>(rtype::MBO),
            order_id: 0,
            price: 100_000_000_000,
            size: 100,
            flags: FlagSet::default(),
            channel_id: 0,
            action: Action::Add as c_char,
            side: databento::dbn::Side::Bid as c_char,
            ts_recv: UNDEF_TIMESTAMP,
            ts_in_delta: 0,
            sequence: 0,
        };

        let instrument = InstrumentIndex(0);
        let time = DateTime::from_timestamp_nanos(u64::MAX as i64).to_utc();

        struct TestCase {
            input: (InstrumentIndex, MboMsg),
            expected: MarketEvent<InstrumentIndex, OrderBookEvent>,
        }

        let test_cases = vec![
            TestCase {
                input: (instrument, mbo.clone()),
                expected:  MarketEvent {
                    time_exchange: time,
                    time_received: time,
                    exchange: ExchangeId::Other,
                    instrument,
                    kind: OrderBookEvent::IncrementalUpdate(OrderBookUpdate {
                        order_id: Some(mbo.order_id.to_string()),
                        price: Decimal::from_f64(100.00).unwrap(),
                        amount: Decimal::from(mbo.size),
                        side: Side::Buy,
                        sequence: mbo.sequence as u64,
                        action: OrderBookAction::Add,
                    }),
                },
            },
        ];

        for test_case in test_cases {
            let result = MarketEvent::try_from(test_case.input).unwrap();
            assert_eq!(result, test_case.expected);
        }
    }

    #[test]
    fn test_mbo_to_orderbook_l2() {
        let mbo = Mbp10Msg {
            hd: RecordHeader::default::<MboMsg>(rtype::MBO),
            price: 100_000_000_000,
            size: 100,
            flags: FlagSet::default(),
            action: Action::Add as c_char,
            side: databento::dbn::Side::Bid as c_char,
            ts_recv: UNDEF_TIMESTAMP,
            ts_in_delta: 0,
            sequence: 0,
            depth: 0,
            levels: [
                BidAskPair {
                    bid_px: 100_000_000_000,
                    ask_px: 101_000_000_000,
                    bid_sz: 100,
                    ask_sz: 100,
                    bid_ct: 0,
                    ask_ct: 0,
                },
                BidAskPair {
                    bid_px: 99_000_000_000,
                    ask_px: 102_000_000_000,
                    bid_sz: 100,
                    ask_sz: 100,
                    bid_ct: 0,
                    ask_ct: 0,
                },
                BidAskPair {
                    bid_px: 98_000_000_000,
                    ask_px: UNDEF_PRICE,
                    bid_sz: 100,
                    ask_sz: 100,
                    bid_ct: 0,
                    ask_ct: 0,
                },
                BidAskPair::default(),
                BidAskPair::default(),
                BidAskPair::default(),
                BidAskPair::default(),
                BidAskPair::default(),
                BidAskPair::default(),
                BidAskPair::default(),
            ],
        };

        let instrument = InstrumentIndex(0);
        let time = DateTime::from_timestamp_nanos(u64::MAX as i64).to_utc();

        struct TestCase {
            input: (InstrumentIndex, Mbp10Msg),
            expected: MarketEvent<InstrumentIndex, OrderBookEvent>,
        }

        let test_cases = vec![
            TestCase {
                input: (instrument, mbo.clone()),
                expected:  MarketEvent {
                    time_exchange: time,
                    time_received: time,
                    exchange: ExchangeId::Other,
                    instrument,
                    kind: OrderBookEvent::Update(OrderBook::new(
                        0,
                        Some(time),
                        vec![
                            Level {
                                price: Decimal::from_f64(100.00).unwrap(),
                                amount: Decimal::from(100),
                            },
                            Level {
                                price: Decimal::from_f64(99.00).unwrap(),
                                amount: Decimal::from(100),
                            },
                            Level {
                                price: Decimal::from_f64(98.00).unwrap(),
                                amount: Decimal::from(100),
                            }
                        ],
                        vec![
                            Level {
                                price: Decimal::from_f64(101.00).unwrap(),
                                amount: Decimal::from(100),
                            },
                            Level {
                                price: Decimal::from_f64(102.00).unwrap(),
                                amount: Decimal::from(100),
                            }
                        ]
                    )),
                },
            },
        ];

        for test_case in test_cases {
            let result = MarketEvent::try_from(test_case.input).unwrap();
            assert_eq!(result, test_case.expected);
        }
    }

    #[test]
    fn test_trademsg_to_public_trade() {
        let mbo = TradeMsg {
            hd: RecordHeader::default::<MboMsg>(rtype::MBO),
            price: 100_000_000_000,
            size: 100,
            flags: FlagSet::default(),
            action: Action::Trade as c_char,
            side: databento::dbn::Side::Bid as c_char,
            ts_recv: UNDEF_TIMESTAMP,
            ts_in_delta: 0,
            sequence: 100,
            depth: 0,
        };

        let instrument = InstrumentIndex(0);
        let time = DateTime::from_timestamp_nanos(u64::MAX as i64).to_utc();

        struct TestCase {
            input: (InstrumentIndex, TradeMsg),
            expected: MarketEvent<InstrumentIndex, PublicTrade>,
        }

        let test_cases = vec![
            TestCase {
                input: (instrument, mbo.clone()),
                expected:  MarketEvent {
                    time_exchange: time,
                    time_received: time,
                    exchange: ExchangeId::Other,
                    instrument,
                    kind: PublicTrade {
                        id: "100".to_string(),
                        price: 100.00,
                        amount: 100.0,
                        side: Side::Buy,
                    },
                },
            },
        ];

        for test_case in test_cases {
            let result = MarketEvent::try_from(test_case.input).unwrap();
            assert_eq!(result, test_case.expected);
        }
    }
}

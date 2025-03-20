use chrono::DateTime;
use databento::dbn::{Action, MboMsg, RecordRef, UNDEF_PRICE};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use tracing::info;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::instrument::InstrumentIndex;
use barter_instrument::Side;
use crate::error::DataError;
use crate::event::{DataKind, MarketEvent};
use crate::provider::databento::DatabentoSide;
use crate::subscription::book::{OrderBookAction, OrderBookEvent, OrderBookUpdate};

pub fn transform_mb0(mbo: &MboMsg) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {

    let time_exchange = DateTime::from_timestamp_nanos(mbo.ts_recv as i64).to_utc();

    if mbo.flags.is_snapshot() && !mbo.flags.is_last() {
        return match mbo.action() {
            Ok(Action::Add) => {
                let side = mbo.side()?;
                let price = mbo.price_f64();

                info!("Snapshot received {snapshot} {action} {price} {size} {side}",
                    snapshot = true, action = "add", price = price, size = mbo.size, side = Side::from(DatabentoSide::from(side)));

                Ok(Some(MarketEvent {
                    time_exchange: time_exchange.clone(),
                    time_received: chrono::Utc::now(),
                    exchange: ExchangeId::Other,
                    instrument: InstrumentIndex(0),
                    kind: DataKind::from(OrderBookEvent::IncrementalUpdate(OrderBookUpdate {
                        order_id: Some(mbo.order_id.to_string()),
                        price: Decimal::from_f64(price).unwrap(),
                        amount: Decimal::from(mbo.size),
                        side: Side::from(DatabentoSide::from(side)),
                        sequence: mbo.sequence as u64,
                        action: OrderBookAction::Add,
                    })),
                }))
            }
            _ => {
                return Ok(None)
            }
        }
    }

    if mbo.price == UNDEF_PRICE {
        return Ok(None)
    }

    let side = mbo.side()?;
    let price = mbo.price_f64();

    match mbo.action() {
        Ok(Action::Add) => {
            Ok(Some(MarketEvent {
                time_exchange: time_exchange.clone(),
                time_received: chrono::Utc::now(),
                exchange: ExchangeId::Other,
                instrument: InstrumentIndex(0),
                kind: DataKind::from(OrderBookEvent::IncrementalUpdate(OrderBookUpdate {
                    order_id: Some(mbo.order_id.to_string()),
                    price: Decimal::from_f64(price).unwrap(),
                    amount: Decimal::from(mbo.size),
                    side: Side::from(DatabentoSide::from(side)),
                    sequence: mbo.sequence as u64,
                    action: OrderBookAction::Add,
                })),
            }))
        }
        Ok(Action::Modify) => {
            Ok(Some(MarketEvent {
                time_exchange: time_exchange.clone(),
                time_received: chrono::Utc::now(),
                exchange: ExchangeId::Other,
                instrument: InstrumentIndex(0),
                kind: DataKind::from(OrderBookEvent::IncrementalUpdate(OrderBookUpdate {
                    order_id: Some(mbo.order_id.to_string()),
                    price: Decimal::from_f64(price).unwrap(),
                    amount: Decimal::from(mbo.size),
                    side: Side::from(DatabentoSide::from(side)),
                    sequence: mbo.sequence as u64,
                    action: OrderBookAction::Modify,
                })),
            }))
        },
        Ok(Action::Cancel) => {
            Ok(Some(MarketEvent {
                time_exchange: time_exchange.clone(),
                time_received: chrono::Utc::now(),
                exchange: ExchangeId::Other,
                instrument: InstrumentIndex(0),
                kind: DataKind::from(OrderBookEvent::IncrementalUpdate(OrderBookUpdate {
                    order_id: Some(mbo.order_id.to_string()),
                    price: Decimal::from_f64(price).unwrap(),
                    amount: Decimal::from(mbo.size),
                    side: Side::from(DatabentoSide::from(side)),
                    sequence: mbo.sequence as u64,
                    action: OrderBookAction::Cancel,
                })),
            }))
        },
        Ok(Action::Clear) => {
            Ok(Some(MarketEvent {
                time_exchange: time_exchange.clone(),
                time_received: chrono::Utc::now(),
                exchange: ExchangeId::Other,
                instrument: InstrumentIndex(0),
                kind: DataKind::from(OrderBookEvent::Clear),
            }))
        },
        Ok(Action::Trade) | Ok(Action::Fill) | Ok(Action::None) => {
            Ok(None)
        }
        Err(e) => {
            Err(DataError::from(e))
        }
    }
}

pub fn transform(record_ref: RecordRef<'_>) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {
    if let Some(mb0) = record_ref.get::<MboMsg>() {
        return transform_mb0(mb0);
    }
    Ok(None)
}
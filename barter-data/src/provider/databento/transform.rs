use chrono::DateTime;
use databento::dbn::{Action, MboMsg, RecordRef, UNDEF_PRICE};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::instrument::InstrumentIndex;
use barter_instrument::Side;
use crate::error::DataError;
use crate::event::{DataKind, MarketEvent};
use crate::provider::databento::DatabentoSide;
use crate::subscription::book::{OrderBookAction, OrderBookEvent, OrderBookUpdate};

fn to_market_event(mbo: MboMsg, action: OrderBookAction) -> MarketEvent<InstrumentIndex, DataKind> {
    let time_exchange = DateTime::from_timestamp_nanos(mbo.ts_recv as i64).to_utc();
    let time_received = chrono::Utc::now();
    let exchange = ExchangeId::Other;
    let instrument = InstrumentIndex(0);
    let kind = DataKind::from(
        OrderBookEvent::IncrementalUpdate(OrderBookUpdate::from((mbo, action))));

    MarketEvent {
        time_exchange,
        time_received,
        exchange,
        instrument,
        kind,
    }
}

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

pub fn transform_mb0(mbo: &MboMsg) -> Result<Option<MarketEvent<InstrumentIndex, DataKind>>, DataError> {
    if mbo.price == UNDEF_PRICE {
        return Ok(None)
    }

    match mbo.action() {
        Ok(Action::Add) => Ok(Some(to_market_event(mbo.clone(), OrderBookAction::Add))),
        Ok(Action::Modify) => Ok(Some(to_market_event(mbo.clone(), OrderBookAction::Modify))),
        Ok(Action::Cancel) => Ok(Some(to_market_event(mbo.clone(), OrderBookAction::Cancel))),
        Ok(Action::Clear) => {
            Ok(Some(MarketEvent {
                time_exchange: DateTime::from_timestamp_nanos(mbo.ts_recv as i64).to_utc(),
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

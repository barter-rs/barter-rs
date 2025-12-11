//! MEXC L1 OrderBook transformer.

use crate::{
    books::Level,
    error::DataError,
    event::MarketEvent,
    exchange::mexc::{MexcSpot, market::extract_symbol_from_channel, proto::PushDataV3ApiWrapper},
    subscription::{Map, book::OrderBooksL1},
    transformer::ExchangeTransformer,
};
use async_trait::async_trait;
use barter_instrument::exchange::ExchangeId;
use barter_integration::{Transformer, protocol::websocket::WsMessage, subscription::SubscriptionId};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// MEXC L1 orderbook transformer.
///
/// L1 provides complete snapshots of top N levels with each message.
/// No state management required.
#[derive(Debug)]
pub struct MexcOrderBooksL1Transformer<InstrumentKey> {
    instrument_map: HashMap<SubscriptionId, InstrumentKey>,
}

#[async_trait]
impl<InstrumentKey> ExchangeTransformer<MexcSpot, InstrumentKey, OrderBooksL1>
    for MexcOrderBooksL1Transformer<InstrumentKey>
where
    InstrumentKey: Clone + Send,
{
    async fn init(
        instrument_map: Map<InstrumentKey>,
        _initial_snapshots: &[MarketEvent<InstrumentKey, <OrderBooksL1 as crate::subscription::SubscriptionKind>::Event>],
        _ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError> {
        Ok(Self {
            instrument_map: instrument_map.0.into_iter().collect(),
        })
    }
}

impl<InstrumentKey> Transformer for MexcOrderBooksL1Transformer<InstrumentKey>
where
    InstrumentKey: Clone,
{
    type Error = DataError;
    type Input = PushDataV3ApiWrapper;
    type Output = MarketEvent<InstrumentKey, <OrderBooksL1 as crate::subscription::SubscriptionKind>::Event>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        let Some(symbol) = extract_symbol_from_channel(&input.channel) else {
            return vec![];
        };

        let sub_id = SubscriptionId::from(format!("limit.depth|{}", symbol));
        let Some(instrument_key) = self.instrument_map.get(&sub_id).cloned() else {
            return vec![];
        };

        let Some(limit_depth) = input.public_limit_depths else {
            return vec![];
        };

        let ts = input
            .send_time
            .or(input.create_time)
            .and_then(DateTime::from_timestamp_millis)
            .unwrap_or_else(Utc::now);

        let parse_level = |level: &crate::exchange::mexc::proto::PublicLimitDepthV3ApiItem| {
            let price: Decimal = level.price.parse().ok()?;
            let amount: Decimal = level.quantity.parse().ok()?;
            (!price.is_zero()).then(|| Level::new(price, amount))
        };

        vec![Ok(MarketEvent {
            time_exchange: ts,
            time_received: Utc::now(),
            exchange: ExchangeId::Mexc,
            instrument: instrument_key,
            kind: crate::subscription::book::OrderBookL1 {
                last_update_time: ts,
                best_bid: limit_depth.bids.first().and_then(parse_level),
                best_ask: limit_depth.asks.first().and_then(parse_level),
            },
        })]
    }
}



use super::ExchangeTransformer;
use crate::{
    error::DataError,
    event::{MarketEvent, MarketIter},
    exchange::{Connector, ExchangeId},
    subscription::{Map, SubscriptionKind},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{model::SubscriptionId, protocol::websocket::WsMessage, Transformer};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use tokio::sync::mpsc;

/// Standard generic stateless [`ExchangeTransformer`] to translate exchange specific types into
/// normalised Barter types. Often used with
/// [`PublicTrades`](crate::subscription::trade::PublicTrades) or
/// [`OrderBooksL1`](crate::subscription::book::OrderBooksL1) streams.
#[derive(Clone, Eq, PartialEq, Debug, Serialize)]
pub struct StatelessTransformer<Exchange, InstrumentId, Kind, Input> {
    instrument_map: Map<InstrumentId>,
    phantom: PhantomData<(Exchange, Kind, Input)>,
}

#[async_trait]
impl<Exchange, InstrumentId, Kind, Input> ExchangeTransformer<Exchange, InstrumentId, Kind>
    for StatelessTransformer<Exchange, InstrumentId, Kind, Input>
where
    Exchange: Connector + Send,
    InstrumentId: Clone + Send,
    Kind: SubscriptionKind + Send,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<InstrumentId, Kind::Event>: From<(ExchangeId, InstrumentId, Input)>,
{
    async fn new(
        _: mpsc::UnboundedSender<WsMessage>,
        instrument_map: Map<InstrumentId>,
    ) -> Result<Self, DataError> {
        Ok(Self {
            instrument_map,
            phantom: PhantomData,
        })
    }
}

impl<Exchange, InstrumentId, Kind, Input> Transformer
    for StatelessTransformer<Exchange, InstrumentId, Kind, Input>
where
    Exchange: Connector,
    InstrumentId: Clone,
    Kind: SubscriptionKind,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<InstrumentId, Kind::Event>: From<(ExchangeId, InstrumentId, Input)>,
{
    type Error = DataError;
    type Input = Input;
    type Output = MarketEvent<InstrumentId, Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        // Determine if the message has an identifiable SubscriptionId
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        // Find Instrument associated with Input and transform
        match self.instrument_map.find(&subscription_id) {
            Ok(instrument) => {
                MarketIter::<InstrumentId, Kind::Event>::from((
                    Exchange::ID,
                    instrument.clone(),
                    input,
                ))
                .0
            }
            Err(unidentifiable) => vec![Err(DataError::Socket(unidentifiable))],
        }
    }
}

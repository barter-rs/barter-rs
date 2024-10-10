use crate::{
    error::DataError,
    event::MarketEvent,
    subscription::{Map, SubscriptionKind},
};
use async_trait::async_trait;
use barter_integration::{protocol::websocket::WsMessage, Transformer};
use tokio::sync::mpsc;

/// Generic stateless [`ExchangeTransformer`] often used for transforming
/// [`PublicTrades`](crate::subscription::trade::PublicTrades) streams.
pub mod stateless;

/// Defines how to construct a [`Transformer`] used by [`MarketStream`](super::MarketStream)s to
/// translate exchange specific types to normalised Barter types.
#[async_trait]
pub trait ExchangeTransformer<Exchange, InstrumentKey, Kind>
where
    Self: Transformer<Output = MarketEvent<InstrumentKey, Kind::Event>, Error = DataError> + Sized,
    Kind: SubscriptionKind,
{
    /// Construct a new [`Self`].
    ///
    /// The [`mpsc::UnboundedSender`] can be used by [`Self`] to send messages back to the exchange.
    async fn new(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        instrument_map: Map<InstrumentKey>,
    ) -> Result<Self, DataError>;
}

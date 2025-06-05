use crate::{
    error::DataError,
    event::MarketEvent,
    subscription::{Map, SubscriptionKind},
};
use async_trait::async_trait;
use barter_integration::{Transformer, protocol::websocket::WsMessage};
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
    /// Initialise a new [`Self`], also fetching any market data snapshots required for the
    /// associated Exchange and SubscriptionKind market stream to function.
    ///
    /// The [`mpsc::UnboundedSender`] can be used by [`Self`] to send messages back to the exchange.
    async fn init(
        instrument_map: Map<InstrumentKey>,
        initial_snapshots: &[MarketEvent<InstrumentKey, Kind::Event>],
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
    ) -> Result<Self, DataError>;
}

use self::{
    mapper::{SubscriptionMapper, WebSocketSubMapper},
    validator::SubscriptionValidator,
};
use crate::{
    exchange::Connector,
    instrument::InstrumentData,
    subscription::{Map, Subscription, SubscriptionKind, SubscriptionMeta},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{connect, WebSocket},
};
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// [`SubscriptionMapper`] implementations defining how to map a
/// collection of Barter [`Subscription`]s into exchange specific [`SubscriptionMeta`].
pub mod mapper;

/// [`SubscriptionValidator`] implementations defining how to
/// validate actioned [`Subscription`]s were successful.
pub mod validator;

/// Defines how to connect to a socket and subscribe to market data streams.
#[async_trait]
pub trait Subscriber {
    type SubMapper: SubscriptionMapper;

    async fn subscribe<Exchange, Instrument, Kind>(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> Result<(WebSocket, Map<Instrument::Id>), SocketError>
    where
        Exchange: Connector + Send + Sync,
        Kind: SubscriptionKind + Send + Sync,
        Instrument: InstrumentData,
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

/// Standard [`Subscriber`] for [`WebSocket`]s suitable for most exchanges.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct WebSocketSubscriber;

#[async_trait]
impl Subscriber for WebSocketSubscriber {
    type SubMapper = WebSocketSubMapper;

    async fn subscribe<Exchange, Instrument, Kind>(
        subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> Result<(WebSocket, Map<Instrument::Id>), SocketError>
    where
        Exchange: Connector + Send + Sync,
        Kind: SubscriptionKind + Send + Sync,
        Instrument: InstrumentData,
        Subscription<Exchange, Instrument, Kind>:
            Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
    {
        // Define variables for logging ergonomics
        let exchange = Exchange::ID;
        let url = Exchange::url()?;
        debug!(%exchange, %url, ?subscriptions, "subscribing to WebSocket");

        // Connect to exchange
        let mut websocket = connect(url).await?;
        debug!(%exchange, ?subscriptions, "connected to WebSocket");

        // Map &[Subscription<Exchange, Kind>] to SubscriptionMeta
        let SubscriptionMeta {
            instrument_map,
            subscriptions,
        } = Self::SubMapper::map::<Exchange, Instrument, Kind>(subscriptions);

        // Send Subscriptions over WebSocket
        for subscription in subscriptions {
            debug!(%exchange, payload = ?subscription, "sending exchange subscription");
            websocket.send(subscription).await?;
        }

        // Validate Subscription responses
        let map = Exchange::SubValidator::validate::<Exchange, Instrument, Kind>(
            instrument_map,
            &mut websocket,
        )
        .await?;

        info!(%exchange, "subscribed to WebSocket");
        Ok((websocket, map))
    }
}

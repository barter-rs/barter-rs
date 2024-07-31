use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    protocol::websocket::{connect_local, WebSocket, WsMessage},
};
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info};

use crate::{
    exchange::{ibkr::IbkrRest, Connector}, instrument::InstrumentData, subscriber::{
        mapper::{SubscriptionMapper, WebSocketSubMapper},
        validator::SubscriptionValidator,
        Subscriber,
    }, subscription::{Map, Subscription, SubscriptionKind, SubscriptionMeta}, Identifier
};

/// [`Ibkr`] [`Subscriber`] for [`WebSocket`]s.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct IbkrWebSocketSubscriber;

#[async_trait]
impl Subscriber for IbkrWebSocketSubscriber {
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
        debug!(%exchange, %url, "subscribing to WebSocket");

        // Connect to exchange
        let mut websocket = connect_local(url).await?;
        debug!(%exchange, "connected to WebSocket");

        // Authorize the session
        let ibkr_http = IbkrRest::new()?;
        let session_id = ibkr_http.get_session().await?;
        let session_message = json!({"session": session_id}).to_string();
        debug!(%exchange, ?session_id, ?session_message);

        let authz_message = WsMessage::text::<String>(json!({"session": session_id}).to_string());
        websocket.send(authz_message).await?;
        websocket.flush().await?; // flush authz msg before attempting subscriptions

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
        let map =
            Exchange::SubValidator::validate::<Exchange, Instrument, Kind>(instrument_map, &mut websocket)
                .await?;

        info!(%exchange, "subscribed to WebSocket");
        Ok((websocket, map))
    }
}

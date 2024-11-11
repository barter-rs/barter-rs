use std::{sync::Arc, thread, time::Duration};

use crate::{
    exchange::ibkr::{Connector, IbkrRest, IbkrWebSocketRequest},
    instrument::InstrumentData,
    subscriber::{
        mapper::{SubscriptionMapper, WebSocketSubMapper},
        validator::SubscriptionValidator,
        Connect, Subscribed, Subscriber,
    },
    subscription::{Subscription, SubscriptionKind, SubscriptionMeta},
    Identifier,
};
use async_trait::async_trait;
use barter_integration::{
    error::SocketError,
    protocol::{danger::NoCertificateVerification, websocket::WebSocket},
};
use futures::SinkExt;
use rustls::{ClientConfig, RootCertStore};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::client::IntoClientRequest};
use tracing::info;

/// [`Ibkr`] [`Subscriber`] for [`WebSocket`]s.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct IbkrWebSocketSubscriber;

/// [`Ibkr`] [`Connect`] for [`WebSocketSubscriber`].
/// Ibkr connects to a locally-running gateway, which acts as a proxy to the exchange
/// for both http and websocket requests.
/// Because the gateway is local, this trait implementation disables certificate verification.
#[async_trait]
impl Connect for IbkrWebSocketSubscriber {
    async fn connect<R>(request: R) -> Result<WebSocket, SocketError>
    where
        R: IntoClientRequest + Send + Unpin + Debug,
    {
        // debug!(?request, "attempting to establish WebSocket connection");
        let root_cert_store = RootCertStore::empty();

        let mut config = ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        config
            .dangerous()
            .set_certificate_verifier(Arc::new(NoCertificateVerification {}));

        let connector = tokio_tungstenite::Connector::Rustls(Arc::new(config));
        connect_async_tls_with_config(request, None, false, Some(connector))
            .await
            .map(|(websocket, _)| websocket)
            .map_err(SocketError::WebSocket)
    }
}

// TODO: I tried to reuse WebSocketSubscriber impl, but didn't quite get
//       all of the connect() stuff extracted properly.
#[async_trait]
impl Subscriber for IbkrWebSocketSubscriber {
    type SubMapper = WebSocketSubMapper;

    async fn subscribe<Exchange, Instrument, Kind>(
        ws_subscriptions: &[Subscription<Exchange, Instrument, Kind>],
    ) -> Result<Subscribed<Instrument::Key>, SocketError>
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
        info!(%exchange, %url, "connecting to WebSocket");

        // Authorize the session (via REST call)
        let ibkr_rest = IbkrRest::new()?;
        let session_id = ibkr_rest.get_session().await?;

        // get websocket request with session_id cookie
        let ws_request = IbkrWebSocketRequest::new(url, session_id.clone()).request();

        // Connect to exchange via Ibkr Client Gateway (running on localhost)
        let mut websocket = Self::connect(ws_request).await?;
        info!(%exchange, "connected to WebSocket");

        // Map &[Subscription<Exchange, Kind>] to SubscriptionMeta
        let SubscriptionMeta {
            instrument_map,
            ws_subscriptions,
        } = Self::SubMapper::map::<Exchange, Instrument, Kind>(ws_subscriptions);

        // dunno yet why this is needed... but it is.
        let pause = Duration::from_millis(500);
        thread::sleep(pause);

        // Send Subscriptions over WebSocket
        for subscription in ws_subscriptions {
            info!(%exchange, payload = ?subscription, "sending exchange subscription");
            websocket.feed(subscription).await?;
        }
        websocket.flush().await?;

        // Validate Subscription responses
        let (map, buffered_websocket_events) = Exchange::SubValidator::validate::<
            Exchange,
            Instrument::Key,
            Kind,
        >(instrument_map, &mut websocket)
        .await?;

        info!(%exchange, "subscribed to WebSocket");
        Ok(Subscribed {
            websocket,
            map,
            buffered_websocket_events,
        })
    }
}

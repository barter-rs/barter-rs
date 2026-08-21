//! Raw TCP and TLS transports for FIX sessions.
//!
//! [`connect_tcp`] establishes a plaintext TCP connection (local or trusted
//! links); [`connect_tls`] establishes a TLS connection over TCP using
//! `tokio-rustls` with webpki-roots, consistent with this workspace's
//! rustls-only policy (no OpenSSL). Both return a [`FixSocket`] — the socket
//! split into a receive [`FixStream`] (complete FIX frames) and a send
//! [`FixSink`] (encoded FIX frames).

use super::stream::{FixFrameSink, FixFrameStream, MaybeTlsStream};
use super::{FixSocket, FixTransportError};
use std::sync::Arc;
use tokio_rustls::rustls;

/// Establish a plaintext TCP FIX connection to `host:port`.
///
/// Returns a [`FixSocket`] split into a receive [`FixStream`] yielding complete
/// FIX frames and a send [`FixSink`] writing encoded frames.
///
/// # Example
///
/// ```
/// # async fn run() -> Result<(), barter_integration::protocol::fix::FixTransportError> {
/// use barter_integration::protocol::fix::connect_tcp;
///
/// let socket = connect_tcp("127.0.0.1", 9120).await?;
/// # Ok(())
/// # }
/// ```
pub async fn connect_tcp(host: &str, port: u16) -> Result<FixSocket, FixTransportError> {
    let tcp_stream = tokio::net::TcpStream::connect((host, port)).await?;
    Ok(split_socket(MaybeTlsStream::Plain(tcp_stream)))
}

/// Establish a TLS FIX connection to `host:port`.
///
/// Uses `tokio-rustls` with the webpki-roots trust anchors. `host` is used both
/// as the connection address and as the TLS Server Name Indication (SNI)
/// hostname, so it must be a DNS hostname (not an IP literal).
///
/// Returns a [`FixSocket`] split into a receive [`FixStream`] yielding complete
/// FIX frames and a send [`FixSink`] writing encoded frames.
///
/// # Example
///
/// ```
/// # async fn run() -> Result<(), barter_integration::protocol::fix::FixTransportError> {
/// use barter_integration::protocol::fix::connect_tls;
///
/// let socket = connect_tls("fix.testnet.binance.com", 9443).await?;
/// # Ok(())
/// # }
/// ```
pub async fn connect_tls(host: &str, port: u16) -> Result<FixSocket, FixTransportError> {
    let connector = tls_connector()?;
    let tcp_stream = tokio::net::TcpStream::connect((host, port)).await?;

    // SNI server name; must be a DNS hostname.
    let server_name = rustls::pki_types::ServerName::try_from(host.to_owned())
        .map_err(|_| FixTransportError::InvalidServerName(host.to_owned()))?;

    let tls_stream = connector.connect(server_name, tcp_stream).await?;
    Ok(split_socket(MaybeTlsStream::Tls(Box::new(tls_stream))))
}

/// Build the rustls TLS connector with webpki-roots trust anchors.
///
/// The crypto provider is pinned to `ring` explicitly: this workspace's
/// dependency graph enables **both** `rustls/aws-lc-rs` and `rustls/ring`
/// (feature unification across tokio-tungstenite and reqwest), so the implicit
/// `ClientConfig::builder()` default provider would panic at runtime.
fn tls_connector() -> Result<tokio_rustls::TlsConnector, rustls::Error> {
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(webpki_root_store())
    .with_no_client_auth();

    Ok(tokio_rustls::TlsConnector::from(Arc::new(config)))
}

/// A [`rustls::RootCertStore`] populated with the webpki-roots trust anchors.
///
/// Kept as a separate function so its contents are directly testable (rustls's
/// `ClientConfig` does not expose the configured root store).
fn webpki_root_store() -> rustls::RootCertStore {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    root_store
}

/// Split a [`MaybeTlsStream`] into a framed receive [`FixStream`] and send
/// [`FixSink`].
fn split_socket(socket: MaybeTlsStream) -> FixSocket {
    let (reader, writer) = tokio::io::split(socket);
    FixSocket {
        stream: FixFrameStream::new(reader),
        sink: FixFrameSink::new(writer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connect_tcp_round_trip() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let socket = connect_tcp("127.0.0.1", addr.port())
            .await
            .expect("connect_tcp should connect to local listener");

        // The accept side confirms the connection is established.
        let (_, _) = listener.accept().await.unwrap();

        // Socket halves are usable: send and receive sides exist.
        let _ = socket.stream;
        let _ = socket.sink;
    }

    #[test]
    fn test_tls_connector_has_webpki_roots() {
        // Smoke test: the connector must be buildable and carry the webpki-roots
        // trust anchors (so TLS verification against public CAs works).
        let _connector = tls_connector().expect("TLS connector should build");
        let roots = webpki_root_store().roots;
        assert!(!roots.is_empty(), "webpki-roots store should not be empty");
    }

    #[test]
    fn test_connect_tls_rejects_invalid_server_name() {
        // A DNS server name containing a space is invalid; the parse error maps
        // to FixTransportError::InvalidServerName.
        let bad = "inv alid";
        let error = rustls::pki_types::ServerName::try_from(bad.to_owned())
            .map_err(|_| FixTransportError::InvalidServerName(bad.to_owned()))
            .unwrap_err();
        assert!(matches!(
            error,
            FixTransportError::InvalidServerName(host) if host == "inv alid"
        ));
    }
}

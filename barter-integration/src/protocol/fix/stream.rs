//! Framed FIX stream and sink adapters over an underlying `AsyncRead`/
//! `AsyncWrite` socket, plus the [`MaybeTlsStream`] plaintext-or-TLS socket
//! abstraction.
//!
//! The framing itself is pure and deterministic ([`FrameAccumulator`]); these
//! adapters wire it to Tokio I/O.

use super::FixTransportError;
use super::frame::FrameAccumulator;
use bytes::Bytes;
use futures::{Sink, Stream};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, ReadHalf, WriteHalf};

/// Convenient type alias for the read half of a [`MaybeTlsStream`].
pub type MaybeTlsReadHalf = ReadHalf<MaybeTlsStream>;

/// Convenient type alias for the write half of a [`MaybeTlsStream`].
pub type MaybeTlsWriteHalf = WriteHalf<MaybeTlsStream>;

/// A socket that is either a plaintext [`tokio::net::TcpStream`] or a TLS
/// [`tokio_rustls::client::TlsStream`] over TCP.
///
/// Mirrors tungstenite's `MaybeTlsStream` so a single framing adapter serves
/// both the [`connect_tcp`](super::transport::connect_tcp) and
/// [`connect_tls`](super::transport::connect_tls) transports.
#[derive(Debug)]
pub enum MaybeTlsStream {
    /// Plaintext TCP connection.
    Plain(tokio::net::TcpStream),
    /// TLS connection established via `tokio-rustls`.
    ///
    /// Boxed to keep the enum small (`TlsStream` is ~1 KiB while `TcpStream`
    /// is 48 bytes); the box is allocated once per TLS connection.
    Tls(Box<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>),
}

impl AsyncRead for MaybeTlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for MaybeTlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_shutdown(cx),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            Self::Plain(stream) => stream.is_write_vectored(),
            Self::Tls(stream) => stream.as_ref().is_write_vectored(),
        }
    }
}

/// Read buffer size used when polling the underlying socket.
const READ_BUF_LEN: usize = 8192;

/// A [`Stream`] of complete FIX frames read from an underlying socket.
///
/// Bytes are read from `S` in chunks and fed to a [`FrameAccumulator`], which
/// extracts complete frames by BodyLength. Partial frames are buffered across
/// reads; a frame is only yielded once its full byte length is available.
///
/// Errors: socket I/O failures yield `Err(FixTransportError::Io(_))`; a byte
/// stream that cannot be split into valid frames yields
/// `Err(FixTransportError::Framing(_))` and the buffered bytes are discarded
/// (the caller is expected to resync or reconnect). EOF with no partial frame
/// terminates the stream with `None`.
#[derive(Debug)]
pub struct FixFrameStream<S> {
    inner: S,
    buffer: [u8; READ_BUF_LEN],
    accumulator: FrameAccumulator,
}

impl<S> FixFrameStream<S> {
    /// Construct a [`FixFrameStream`] over the provided socket.
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: [0; READ_BUF_LEN],
            accumulator: FrameAccumulator::new(),
        }
    }
}

impl<S> Stream for FixFrameStream<S>
where
    S: AsyncRead + Unpin,
{
    type Item = Result<Bytes, FixTransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            // Extract a buffered frame if one is complete.
            match this.accumulator.next_frame() {
                Ok(Some(frame)) => return Poll::Ready(Some(Ok(frame))),
                Ok(None) => {}
                Err(error) => {
                    // Misaligned stream: discard and surface the error.
                    this.accumulator.reset();
                    return Poll::Ready(Some(Err(FixTransportError::Framing(error))));
                }
            }

            // Read more bytes from the underlying socket.
            let mut read_buf = ReadBuf::new(&mut this.buffer);
            match Pin::new(&mut this.inner).poll_read(cx, &mut read_buf) {
                Poll::Ready(Ok(())) => {
                    let filled = read_buf.filled().len();
                    if filled == 0 {
                        // Clean EOF: no more frames are coming.
                        return Poll::Ready(None);
                    }
                    this.accumulator.push(read_buf.filled());
                }
                Poll::Ready(Err(error)) => {
                    return Poll::Ready(Some(Err(FixTransportError::Io(error))));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// A [`Sink`] that writes complete FIX frames to an underlying socket.
///
/// Each item is a `Bytes` frame (already encoded, e.g. via
/// [`fix_codec::encode`]); it is written to the socket and flushed. Partial
/// writes are tracked across polls.
#[derive(Debug)]
pub struct FixFrameSink<S> {
    inner: S,
    pending: Option<(Bytes, usize)>,
}

impl<S> FixFrameSink<S> {
    /// Construct a [`FixFrameSink`] over the provided socket.
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            pending: None,
        }
    }
}

impl<S> Sink<Bytes> for FixFrameSink<S>
where
    S: AsyncWrite + Unpin,
{
    type Error = FixTransportError;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.pending.is_none() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Err(FixTransportError::Io(io::Error::new(
                io::ErrorKind::WouldBlock,
                "previous frame not flushed",
            ))))
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        let this = self.get_mut();
        if this.pending.is_some() {
            return Err(FixTransportError::Io(io::Error::new(
                io::ErrorKind::WouldBlock,
                "previous frame not flushed",
            )));
        }
        this.pending = Some((item, 0));
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();

        // Write any pending frame, tracking the offset across partial writes.
        while let Some((bytes, written)) = this.pending.take() {
            if written >= bytes.len() {
                continue;
            }
            match Pin::new(&mut this.inner).poll_write(cx, &bytes[written..]) {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(FixTransportError::Io(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write FIX frame",
                    ))));
                }
                Poll::Ready(Ok(n)) => {
                    this.pending = Some((bytes, written + n));
                }
                Poll::Ready(Err(error)) => {
                    this.pending = Some((bytes, written));
                    return Poll::Ready(Err(FixTransportError::Io(error)));
                }
                Poll::Pending => {
                    this.pending = Some((bytes, written));
                    return Poll::Pending;
                }
            }
        }

        Pin::new(&mut this.inner)
            .poll_flush(cx)
            .map_err(FixTransportError::Io)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => return Poll::Pending,
        }
        Pin::new(&mut self.inner)
            .poll_shutdown(cx)
            .map_err(FixTransportError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{SinkExt, StreamExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    /// Assert the next stream item is exactly `expected` (frames cannot be
    /// compared with `assert_eq!` because `FixTransportError` contains
    /// `io::Error`, which does not implement `PartialEq`).
    fn assert_frame_eq(actual: Option<Result<Bytes, FixTransportError>>, expected: Bytes) {
        match actual {
            Some(Ok(frame)) => assert_eq!(frame, expected),
            other => panic!("expected frame {expected:?}, got {other:?}"),
        }
    }

    fn frame(cl_ord_id: &str) -> Vec<u8> {
        let mut message = fix_codec::Message::new();
        message.push(fix_codec::tags::BEGIN_STRING, "FIX.4.4");
        message.push(fix_codec::tags::MSG_TYPE, "D");
        message.push(fix_codec::tags::MSG_SEQ_NUM, "1");
        message.push(fix_codec::tags::SENDER_COMP_ID, "CLIENT1");
        message.push(fix_codec::tags::TARGET_COMP_ID, "EXECUTOR");
        message.push(11, cl_ord_id);
        fix_codec::encode(&message).unwrap()
    }

    #[tokio::test]
    async fn test_stream_yields_frame_written_in_arbitrary_chunks() {
        let (mut writer, reader) = duplex(8192);
        let mut stream = FixFrameStream::new(reader);
        let bytes = frame("stream1");

        // Write the frame in awkward 3-byte chunks across task yields.
        for chunk in bytes.chunks(3) {
            writer.write_all(chunk).await.unwrap();
            tokio::task::yield_now().await;
        }

        assert_frame_eq(stream.next().await, Bytes::from(bytes));
    }

    #[tokio::test]
    async fn test_stream_two_frames_one_connection() {
        let (mut writer, reader) = duplex(8192);
        let mut stream = FixFrameStream::new(reader);
        let a = frame("a");
        let b = frame("b");

        writer.write_all(&a).await.unwrap();
        writer.write_all(&b).await.unwrap();
        writer.shutdown().await.unwrap();

        assert_frame_eq(stream.next().await, Bytes::from(a));
        assert_frame_eq(stream.next().await, Bytes::from(b));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_stream_garbage_prefix_is_framing_error() {
        let (mut writer, reader) = duplex(8192);
        let mut stream = FixFrameStream::new(reader);

        writer.write_all(b"NOT_A_FIX_HEADER\x01").await.unwrap();

        assert!(matches!(
            stream.next().await,
            Some(Err(FixTransportError::Framing(_)))
        ));
    }

    #[tokio::test]
    async fn test_sink_writes_encoded_frame() {
        let (writer, mut reader) = duplex(8192);
        let mut sink = FixFrameSink::new(writer);
        let bytes = frame("sink1");

        sink.send(Bytes::from(bytes.clone())).await.unwrap();
        sink.flush().await.unwrap();

        // Read exactly the frame length (NOT read_to_end, which would wait for
        // the writer to close and deadlock while the sink is still alive).
        let mut received = vec![0u8; bytes.len()];
        reader.read_exact(&mut received).await.unwrap();
        assert_eq!(received, bytes);
    }

    #[tokio::test]
    async fn test_duplex_socket_round_trip() {
        // Full round trip: sink writes frames into a duplex, stream reads them
        // back as frames — the shape PR B's session will use.
        let (writer, reader) = duplex(8192);
        let mut sink = FixFrameSink::new(writer);
        let mut stream = FixFrameStream::new(reader);
        let bytes = frame("roundtrip");

        sink.send(Bytes::from(bytes.clone())).await.unwrap();
        sink.flush().await.unwrap();
        tokio::task::yield_now().await;

        assert_frame_eq(stream.next().await, Bytes::from(bytes));
    }
}

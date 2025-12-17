use futures::Stream;
use tokio_stream::StreamExt;

/// Merge two Streams and terminate when either Stream terminates. Merged Stream is fused, so will
/// end after the first `None`.
pub fn merge<L, R>(left: L, right: R) -> impl Stream<Item = L::Item>
where
    L: Stream,
    R: Stream<Item = L::Item>,
{
    let left = left
        .map(Some)
        .chain(futures::stream::once(std::future::ready(None)));

    let right = right
        .map(Some)
        .chain(futures::stream::once(std::future::ready(None)));

    left.merge(right).map_while(std::convert::identity).fuse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{Tx, mpsc_unbounded};
    use futures::StreamExt;
    use tokio_test::{assert_pending, assert_ready, assert_ready_eq};

    #[tokio::test]
    async fn test_merge() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = std::task::Context::from_waker(waker);

        let (left_tx, left_rx) = mpsc_unbounded::<&'static str>();
        let (right_tx, right_rx) = mpsc_unbounded::<&'static str>();

        let mut stream = merge(left_rx.into_stream(), right_rx.into_stream());

        assert_pending!(stream.poll_next_unpin(&mut cx));

        left_tx.send("left-1").unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some("left-1"));
        assert_pending!(stream.poll_next_unpin(&mut cx));

        left_tx.send("left-2").unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some("left-2"));
        assert_pending!(stream.poll_next_unpin(&mut cx));

        right_tx.send("right-1").unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some("right-1"));
        assert_pending!(stream.poll_next_unpin(&mut cx));

        left_tx.send("left-3").unwrap();
        right_tx.send("right-2").unwrap();
        assert!(matches!(
            assert_ready!(stream.poll_next_unpin(&mut cx)),
            Some("left-3") | Some("right-2")
        ));
        assert!(matches!(
            assert_ready!(stream.poll_next_unpin(&mut cx)),
            Some("left-3") | Some("right-2")
        ));

        right_tx.send("right-3").unwrap();
        assert_ready_eq!(stream.poll_next_unpin(&mut cx), Some("right-3"));
        assert_pending!(stream.poll_next_unpin(&mut cx));

        drop(left_tx);

        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);

        let _ = right_tx.send("right-3");

        assert_ready_eq!(stream.poll_next_unpin(&mut cx), None);
    }
}

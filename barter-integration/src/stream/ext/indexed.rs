use barter_instrument::index::error::IndexError;
use derive_more::Constructor;
use futures::Stream;
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Type that indexes data structures.
///
/// An example `Indexer` use case is "keying" an event: <br>
/// Unindexed = MarketEvent<MarketDataInstrument, DataKind> <br>
/// Indexed = MarketEvent<InstrumentIndex, DataKind>
pub trait Indexer {
    type Unindexed;
    type Indexed;

    /// Index the input.
    fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, IndexError>;
}

/// Stream adapter that indexes items using an [`Indexer`].
#[derive(Debug, Constructor)]
#[pin_project]
pub struct IndexedStream<Stream, Indexer> {
    #[pin]
    pub stream: Stream,
    pub indexer: Indexer,
}

impl<St, Index> Stream for IndexedStream<St, Index>
where
    St: Stream,
    Index: Indexer<Unindexed = St::Item>,
{
    type Item = Result<Index::Indexed, IndexError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.stream.poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(this.indexer.index(item))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::ext::BarterStreamExt;
    use futures::StreamExt;
    use std::collections::HashMap;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use tokio_test::{assert_pending, assert_ready};

    #[derive(Debug, Clone)]
    struct UnindexedData {
        key: String,
        value: i32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct IndexedData {
        index: usize,
        value: i32,
    }

    struct MapIndexer {
        map: HashMap<String, usize>,
    }

    impl Indexer for MapIndexer {
        type Unindexed = UnindexedData;
        type Indexed = IndexedData;

        fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, IndexError> {
            self.map
                .get(&item.key)
                .map(|&index| IndexedData {
                    index,
                    value: item.value,
                })
                .ok_or_else(|| IndexError::InstrumentIndex(format!("key '{}' not found", item.key)))
        }
    }

    #[tokio::test]
    async fn test_indexed_stream() {
        let waker = futures::task::noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let (tx, rx) = mpsc::unbounded_channel::<UnindexedData>();
        let rx = UnboundedReceiverStream::new(rx);

        let mut map = HashMap::new();
        map.insert("a".to_string(), 0);
        map.insert("b".to_string(), 1);
        map.insert("c".to_string(), 2);

        let mut stream = rx.with_index(MapIndexer { map });

        assert_pending!(stream.poll_next_unpin(&mut cx));

        tx.send(UnindexedData {
            key: "a".to_string(),
            value: 10,
        })
        .unwrap();
        assert_eq!(
            assert_ready!(stream.poll_next_unpin(&mut cx)),
            Some(Ok(IndexedData {
                index: 0,
                value: 10
            }))
        );

        tx.send(UnindexedData {
            key: "b".to_string(),
            value: 20,
        })
        .unwrap();
        assert_eq!(
            assert_ready!(stream.poll_next_unpin(&mut cx)),
            Some(Ok(IndexedData {
                index: 1,
                value: 20
            }))
        );

        drop(tx);
        assert_eq!(assert_ready!(stream.poll_next_unpin(&mut cx)), None);
    }
}

use crate::stream::ext::{
    forward_by::ForwardBy,
    forward_clone_by::ForwardCloneBy,
    indexed::{IndexedStream, Indexer},
};
use futures::Stream;

pub mod forward_by;
mod forward_clone_by;
pub mod indexed;

pub trait StreamExt
where
    Self: Stream,
{
    fn with_timeout<TimeoutHandler>(
        self,
        timeout_next_item: std::time::Duration,
        on_timeout: TimeoutHandler,
    ) -> impl Stream<Item = Self::Item>
    where
        Self: Stream + Sized,
        TimeoutHandler: Fn() + 'static,
    {
        use tokio_stream::StreamExt;
        self.timeout(timeout_next_item)
            .map_while(move |timeout_result| match timeout_result {
                Ok(item) => Some(item),
                Err(_elapsed) => {
                    on_timeout();
                    None
                }
            })
    }

    fn with_index<I>(self, indexer: I) -> IndexedStream<Self, I>
    where
        Self: Stream<Item = I::Unindexed> + Sized,
        I: Indexer,
    {
        IndexedStream::new(self, indexer)
    }

    fn forward_by<A, B, FnPredicate, FnForward>(
        self,
        predicate: FnPredicate,
        forward: FnForward,
    ) -> ForwardBy<Self, FnPredicate, FnForward>
    where
        Self: Stream + Sized,
        FnPredicate: Fn(Self::Item) -> futures::future::Either<A, B>,
        FnForward: FnMut(A) -> Result<(), ()>,
    {
        ForwardBy::new(self, predicate, forward)
    }

    fn forward_clone_by<FnPredicate, FnForward>(
        self,
        predicate: FnPredicate,
        forward: FnForward,
    ) -> ForwardCloneBy<Self, FnPredicate, FnForward>
    where
        Self: Stream + Sized,
        Self::Item: Clone,
        FnPredicate: FnMut(&Self::Item) -> bool,
        FnForward: FnMut(Self::Item) -> Result<(), ()>,
    {
        ForwardCloneBy::new(self, predicate, forward)
    }
}

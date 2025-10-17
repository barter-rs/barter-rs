use crate::{Transformer, error::SocketError, protocol::StreamParser};
use futures::Stream;
use pin_project::pin_project;
use std::{
    collections::VecDeque,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

pub mod indexed;
pub mod merge;

/// An [`ExchangeStream`] is a communication protocol agnostic [`Stream`]. It polls protocol
/// messages from the inner [`Stream`], and transforms them into the desired output data structure.
#[derive(Debug)]
#[pin_project]
pub struct ExchangeStream<Protocol, InnerStream, StreamTransformer>
where
    Protocol: StreamParser<StreamTransformer::Input>,
    InnerStream: Stream,
    StreamTransformer: Transformer,
{
    #[pin]
    pub stream: InnerStream,
    pub transformer: StreamTransformer,
    pub buffer: VecDeque<Result<StreamTransformer::Output, StreamTransformer::Error>>,
    pub protocol_marker: PhantomData<Protocol>,
}

impl<Protocol, InnerStream, StreamTransformer> Stream
    for ExchangeStream<Protocol, InnerStream, StreamTransformer>
where
    Protocol: StreamParser<StreamTransformer::Input>,
    InnerStream: Stream<Item = Result<Protocol::Message, Protocol::Error>> + Unpin,
    StreamTransformer: Transformer,
    StreamTransformer::Error: From<SocketError>,
{
    type Item = Result<StreamTransformer::Output, StreamTransformer::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // Flush Self::Item buffer if it is not currently empty
            if let Some(output) = self.buffer.pop_front() {
                return Poll::Ready(Some(output));
            }

            // Poll inner `Stream` for next the next input protocol message
            let input = match self.as_mut().project().stream.poll_next(cx) {
                Poll::Ready(Some(input)) => input,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            };

            // Parse input protocol message into `ExchangeMessage`
            let exchange_message = match Protocol::parse(input) {
                // `StreamParser` successfully deserialised `ExchangeMessage`
                Some(Ok(exchange_message)) => exchange_message,

                // If `StreamParser` returns an Err pass it downstream
                Some(Err(err)) => return Poll::Ready(Some(Err(err.into()))),

                // If `StreamParser` returns None it's a safe-to-skip message
                None => continue,
            };

            // Transform `ExchangeMessage` into `Transformer::OutputIter`
            // ie/ IntoIterator<Item = Result<Output, SocketError>>
            self.transformer
                .transform(exchange_message)
                .into_iter()
                .for_each(
                    |output_result: Result<StreamTransformer::Output, StreamTransformer::Error>| {
                        self.buffer.push_back(output_result)
                    },
                );
        }
    }
}

impl<Protocol, InnerStream, StreamTransformer>
    ExchangeStream<Protocol, InnerStream, StreamTransformer>
where
    Protocol: StreamParser<StreamTransformer::Input>,
    InnerStream: Stream,
    StreamTransformer: Transformer,
{
    pub fn new(
        stream: InnerStream,
        transformer: StreamTransformer,
        buffer: VecDeque<Result<StreamTransformer::Output, StreamTransformer::Error>>,
    ) -> Self {
        Self {
            stream,
            transformer,
            buffer,
            protocol_marker: PhantomData,
        }
    }
}

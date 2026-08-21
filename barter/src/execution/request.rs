use barter_execution::order::request::{
    OrderRequestCancel, OrderRequestOpen, OrderRequestSnapshot,
};
use barter_instrument::{exchange::ExchangeIndex, instrument::InstrumentIndex};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Represents an `Engine` request to the `ExecutionManager`.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize, From)]
pub enum ExecutionRequest<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    /// Request `ExecutionManager` shutdown.
    Shutdown,

    /// Request to cancel an existing `Order`.
    Cancel(OrderRequestCancel<ExchangeKey, InstrumentKey>),

    /// Request to open a new `Order`.
    Open(OrderRequestOpen<ExchangeKey, InstrumentKey>),

    /// Request the current state of `Order`s.
    Snapshots(Vec<OrderRequestSnapshot<ExchangeKey, InstrumentKey>>),
}

#[derive(Debug)]
#[pin_project::pin_project]
pub struct RequestFuture<Request, ResponseFut> {
    /// Returned paired with the response when this future resolves. Stored as `Option` to move out
    /// on completion rather than clone.
    request: Option<Request>,
    #[pin]
    response_future: ResponseFut,
}

impl<Request, ResponseFut> Future for RequestFuture<Request, ResponseFut>
where
    ResponseFut: Future,
{
    type Output = (Request, ResponseFut::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.response_future.poll(cx).map(|result| {
            (
                this.request.take().expect("the request is always set"),
                result,
            )
        })
    }
}

impl<Request, F> RequestFuture<Request, tokio::time::Timeout<F>>
where
    F: Future,
{
    pub fn new(future: F, timeout: std::time::Duration, request: Request) -> Self {
        Self {
            request: Some(request),
            response_future: tokio::time::timeout(timeout, future),
        }
    }
}

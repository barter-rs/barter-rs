use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
use barter_instrument::exchange::ExchangeIndex;
use barter_instrument::instrument::InstrumentIndex;

// pub trait RequestBuffer<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
//
// }

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RequestBuff<ExchangeKey = ExchangeIndex, InstrumentKey = InstrumentIndex> {
    cancels: Buffer<OrderRequestCancel<ExchangeKey, InstrumentKey>>,
    opens: Buffer<OrderRequestOpen<ExchangeKey, InstrumentKey>>,
}

pub struct Buffer<T>(Vec<T>);

impl<T> Buffer<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, value: T) {
        self.0.push(value);
    }

    pub fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        self.0.extend(values);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T> {
        self.0.drain(..)
    }
}
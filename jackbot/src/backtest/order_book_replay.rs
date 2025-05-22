use jackbot_data::{books::OrderBook, subscription::book::OrderBookEvent};

/// Simple order book replay engine.
///
/// Applies [`OrderBookEvent`]s to an [`OrderBook`] to recreate historical book
/// states during backtests.
#[derive(Debug, Clone)]
pub struct OrderBookReplay {
    book: OrderBook,
}

impl OrderBookReplay {
    /// Create a new replay engine starting from the given snapshot.
    pub fn new(snapshot: OrderBook) -> Self {
        Self { book: snapshot }
    }

    /// Apply the next [`OrderBookEvent`] and return a reference to the updated book.
    pub fn apply(&mut self, event: OrderBookEvent) -> &OrderBook {
        self.book.update(event);
        &self.book
    }

    /// Get a reference to the current order book state.
    pub fn current(&self) -> &OrderBook {
        &self.book
    }
}

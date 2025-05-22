use super::OrderBook;
use jackbot_instrument::exchange::ExchangeId;
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct ExchangeBook {
    pub exchange: ExchangeId,
    pub book: Arc<RwLock<OrderBook>>,
}

#[derive(Clone, Default)]
pub struct OrderBookAggregator {
    books: Vec<ExchangeBook>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArbitrageOpportunity {
    pub buy_exchange: ExchangeId,
    pub sell_exchange: ExchangeId,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub spread: Decimal,
}

impl OrderBookAggregator {
    pub fn new(books: impl IntoIterator<Item = ExchangeBook>) -> Self {
        Self { books: books.into_iter().collect() }
    }

    pub fn add_book(&mut self, book: ExchangeBook) {
        self.books.push(book);
    }

    pub fn best_bid(&self) -> Option<(ExchangeId, Decimal)> {
        self.books
            .iter()
            .filter_map(|eb| {
                eb.book
                    .read()
                    .bids()
                    .levels()
                    .first()
                    .map(|lvl| (eb.exchange, lvl.price))
            })
            .max_by(|a, b| a.1.cmp(&b.1))
    }

    pub fn best_ask(&self) -> Option<(ExchangeId, Decimal)> {
        self.books
            .iter()
            .filter_map(|eb| {
                eb.book
                    .read()
                    .asks()
                    .levels()
                    .first()
                    .map(|lvl| (eb.exchange, lvl.price))
            })
            .min_by(|a, b| a.1.cmp(&b.1))
    }

    pub fn detect_arbitrage(&self, threshold: Decimal) -> Option<ArbitrageOpportunity> {
        let (buy_ex, best_ask) = self.best_ask()?;
        let (sell_ex, best_bid) = self.best_bid()?;

        if sell_ex != buy_ex && best_bid - best_ask > threshold {
            Some(ArbitrageOpportunity {
                buy_exchange: buy_ex,
                sell_exchange: sell_ex,
                buy_price: best_ask,
                sell_price: best_bid,
                spread: best_bid - best_ask,
            })
        } else {
            None
        }
    }

    /// Detect arbitrage and log it using `tracing` if found.
    pub fn monitor_and_detect(&self, threshold: Decimal) -> Option<ArbitrageOpportunity> {
        let opp = self.detect_arbitrage(threshold);
        if let Some(ref o) = opp {
            info!(
                buy_exchange = ?o.buy_exchange,
                sell_exchange = ?o.sell_exchange,
                spread = %o.spread,
                "arbitrage opportunity"
            );
        }
        opp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{Level, OrderBook};
    use rust_decimal_macros::dec;

    fn build_book(bid: Decimal, ask: Decimal) -> Arc<RwLock<OrderBook>> {
        Arc::new(RwLock::new(OrderBook::new(
            0,
            None,
            vec![Level::new(bid, dec!(1))],
            vec![Level::new(ask, dec!(1))],
        )))
    }

    #[test]
    fn detects_simple_arbitrage() {
        let book_a = build_book(dec!(10), dec!(11));
        let book_b = build_book(dec!(12), dec!(13));

        let agg = OrderBookAggregator::new([
            ExchangeBook { exchange: ExchangeId::BinanceSpot, book: book_a },
            ExchangeBook { exchange: ExchangeId::Coinbase, book: book_b },
        ]);

        let opp = agg.detect_arbitrage(dec!(0)).expect("should detect");
        assert_eq!(opp.buy_exchange, ExchangeId::BinanceSpot);
        assert_eq!(opp.sell_exchange, ExchangeId::Coinbase);
        assert_eq!(opp.buy_price, dec!(11));
        assert_eq!(opp.sell_price, dec!(12));
    }

    #[test]
    fn no_arbitrage_below_threshold() {
        let book_a = build_book(dec!(10), dec!(11));
        let book_b = build_book(dec!(11.4), dec!(12));

        let agg = OrderBookAggregator::new([
            ExchangeBook { exchange: ExchangeId::BinanceSpot, book: book_a },
            ExchangeBook { exchange: ExchangeId::Coinbase, book: book_b },
        ]);

        assert!(agg.detect_arbitrage(dec!(0.5)).is_none());
    }
}

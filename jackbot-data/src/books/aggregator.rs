use super::{Level, OrderBook};
use jackbot_instrument::exchange::ExchangeId;
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct ExchangeBook {
    /// Exchange identifier for this book.
    pub exchange: ExchangeId,
    /// Shared reference to the associated order book.
    pub book: Arc<RwLock<OrderBook>>,
    /// Weight applied to this book when aggregating volumes.
    pub weight: Decimal,
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

    /// Aggregate order books from all exchanges into a single `OrderBook` snapshot.
    /// `depth` controls how many levels to take from each side after aggregation.
    pub fn aggregate(&self, depth: usize) -> OrderBook {
        use itertools::Itertools;

        let mut bids: Vec<(Decimal, Decimal)> = Vec::new();
        let mut asks: Vec<(Decimal, Decimal)> = Vec::new();
        for eb in &self.books {
            let book = eb.book.read();
            bids.extend(
                book
                    .bids()
                    .levels()
                    .iter()
                    .map(|lvl| (lvl.price, lvl.amount * eb.weight)),
            );
            asks.extend(
                book
                    .asks()
                    .levels()
                    .iter()
                    .map(|lvl| (lvl.price, lvl.amount * eb.weight)),
            );
        }

        let mut merged_bids: Vec<Level> = bids
            .into_iter()
            .into_group_map_by(|(price, _)| *price)
            .into_iter()
            .map(|(price, entries)| {
                let amount: Decimal = entries.into_iter().map(|(_, amt)| amt).sum();
                Level::new(price, amount)
            })
            .collect();
        merged_bids.sort_by(|a, b| b.price.cmp(&a.price));
        merged_bids.truncate(depth);

        let mut merged_asks: Vec<Level> = asks
            .into_iter()
            .into_group_map_by(|(price, _)| *price)
            .into_iter()
            .map(|(price, entries)| {
                let amount: Decimal = entries.into_iter().map(|(_, amt)| amt).sum();
                Level::new(price, amount)
            })
            .collect();
        merged_asks.sort_by(|a, b| a.price.cmp(&b.price));
        merged_asks.truncate(depth);

        OrderBook::new(0, None, merged_bids, merged_asks)
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
            ExchangeBook { exchange: ExchangeId::BinanceSpot, book: book_a, weight: Decimal::ONE },
            ExchangeBook { exchange: ExchangeId::Coinbase, book: book_b, weight: Decimal::ONE },
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
            ExchangeBook { exchange: ExchangeId::BinanceSpot, book: book_a, weight: Decimal::ONE },
            ExchangeBook { exchange: ExchangeId::Coinbase, book: book_b, weight: Decimal::ONE },
        ]);

        assert!(agg.detect_arbitrage(dec!(0.5)).is_none());
    }

    #[test]
    fn aggregates_books_by_weight() {
        let book_a = build_book(dec!(10), dec!(11));
        let book_b = build_book(dec!(12), dec!(13));

        let agg = OrderBookAggregator::new([
            ExchangeBook { exchange: ExchangeId::BinanceSpot, book: book_a, weight: dec!(2) },
            ExchangeBook { exchange: ExchangeId::Coinbase, book: book_b, weight: Decimal::ONE },
        ]);

        let merged = agg.aggregate(1);
        assert_eq!(merged.bids().levels()[0].amount, dec!(3));
        assert_eq!(merged.asks().levels()[0].amount, dec!(3));
    }
}

use crate::{
    model::trade::{SymbolFees, Trade, TradeId},
    ExecutionError, Open, Order, OrderId, RequestOpen,
};
use barter_data::subscription::trade::PublicTrade;
use barter_integration::model::{instrument::Instrument, Side};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

/// [`ClientAccount`](super::ClientAccount) [`Orders`] for each [`Instrument`].
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct ClientOrders {
    pub request_counter: u64,
    pub all: HashMap<Instrument, Orders>,
}

impl ClientOrders {
    /// Construct a new [`ClientOrders`] from the provided selection of [`Instrument`]s.
    pub fn new(instruments: Vec<Instrument>) -> Self {
        Self {
            request_counter: 0,
            all: instruments
                .into_iter()
                .map(|instrument| (instrument, Orders::default()))
                .collect(),
        }
    }

    /// Return a mutable reference to the client [`Orders`] of the specified [`Instrument`].
    pub fn orders_mut(&mut self, instrument: &Instrument) -> Result<&mut Orders, ExecutionError> {
        self.all.get_mut(instrument).ok_or_else(|| {
            ExecutionError::Simulated(format!(
                "SimulatedExchange is not configured for Instrument: {instrument}"
            ))
        })
    }

    /// Fetch the bid and ask [`Order<Open>`]s for every [`Instrument`].
    pub fn fetch_all(&self) -> Vec<Order<Open>> {
        self.all
            .values()
            .flat_map(|market| [&market.bids, &market.asks])
            .flatten()
            .cloned()
            .collect()
    }

    /// Build an [`Order<Open>`] from the provided [`Order<RequestOpen>`]. The request counter
    /// is incremented and the new total is used as a unique [`OrderId`].
    pub fn build_order_open(&mut self, request: Order<RequestOpen>) -> Order<Open> {
        self.increment_request_counter();
        Order::from((self.order_id(), request))
    }

    /// Increment the [`Order<RequestOpen>`] counter by one to ensure the next generated
    /// [`OrderId`] is unique.
    pub fn increment_request_counter(&mut self) {
        self.request_counter += 1;
    }

    /// Generate a unique [`OrderId`].
    pub fn order_id(&self) -> OrderId {
        OrderId(self.request_counter.to_string())
    }
}

/// Client [`Orders`] for an [`Instrument`]. Simulates client orders in an real
/// multi-participant OrderBook.
#[derive(Clone, Eq, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct Orders {
    pub trade_counter: u64,
    pub bids: Vec<Order<Open>>,
    pub asks: Vec<Order<Open>>,
}

impl Orders {
    /// Add an [`Order<Open>`] to the bids or asks depending on it's [`Side`].
    pub fn add_order_open(&mut self, open: Order<Open>) {
        match open.side {
            Side::Buy => {
                // Add Order<Open> to open bids
                self.bids.push(open);
                self.bids.sort();
            }
            Side::Sell => {
                // Add Order<Open> to open asks
                self.asks.push(open);
                self.asks.sort();
            }
        }
    }

    /// Check if an input [`PublicTrade`] matches an bid or ask client [`Open<Order>`].
    ///
    /// Note:
    ///  - In the event that the client has opened both a bid and ask [`Order<Open>`] at the same
    ///    price, preferentially select the Order<Open> with the larger remaining quantity to
    ///    match on.
    pub fn has_matching_order(&self, trade: &PublicTrade) -> Option<Side> {
        match (self.bids.last(), self.asks.last()) {
            // Check the best bid & ask Order<Open> for a match
            (Some(best_bid), Some(best_ask)) => {
                // Note:
                // In the unlikely case that: best_bid.price == best_ask.price == trade.price
                // Preferentially select the larger remaining quantity Order<Open> to match on
                if best_bid.state.price == trade.price && best_ask.state.price == trade.price {
                    let best_bid_quantity = best_bid.state.remaining_quantity();
                    let best_ask_quantity = best_ask.state.remaining_quantity();
                    match best_bid_quantity.partial_cmp(&best_ask_quantity) {
                        Some(Ordering::Greater) => Some(Side::Buy),
                        _ => Some(Side::Sell),
                    }
                }
                // Best bid matches
                else if best_bid.state.price >= trade.price {
                    Some(Side::Buy)
                }
                // Best ask matches
                else if best_ask.state.price <= trade.price {
                    Some(Side::Sell)
                }
                // No matches
                else {
                    None
                }
            }

            // Best bid Order<Open> matches the input PublicTrade
            (Some(best_bid), None) if best_bid.state.price >= trade.price => Some(Side::Buy),

            // Best ask Order<Open> matches the input PublicTrade
            (None, Some(best_ask)) if best_ask.state.price <= trade.price => Some(Side::Sell),

            // Either no bid or ask Order<Open>, or no matches
            _ => None,
        }
    }

    /// Simulates [`Side::Buy`] trades by using the [`PublicTrade`] liquidity to match on open
    /// client bid [`Order<Open>`]s.
    pub fn match_bids(&mut self, trade: &PublicTrade, fees_percent: f64) -> Vec<Trade> {
        // Keep track of how much trade liquidity is remaining to match with
        let mut remaining_liquidity = trade.amount;

        // Collection of execution Trades generated from Order<Open> matches
        let mut trades = vec![];

        let remaining_best_bid = loop {
            // Pop the best bid Order<Open>
            let mut best_bid = match self.bids.pop() {
                Some(best_bid) => best_bid,
                None => break None,
            };

            // Break with remaining best bid if it's not a match, or trade liquidity is exhausted
            if best_bid.state.price < trade.price || remaining_liquidity <= 0.0 {
                break Some(best_bid);
            }

            // Remaining liquidity is either a full-fill or a partial-fill
            self.trade_counter += 1;
            match OrderFill::kind(&best_bid, remaining_liquidity) {
                // Full Order<Open> fill
                OrderFill::Full => {
                    // Remove trade quantity from remaining liquidity
                    let trade_quantity = best_bid.state.remaining_quantity();
                    remaining_liquidity -= trade_quantity;

                    // Generate execution Trade from full Order<Open> fill
                    trades.push(self.generate_trade(best_bid, trade_quantity, fees_percent));

                    // If exact full fill with zero remaining liquidity (highly unlikely), break
                    if remaining_liquidity == 0.0 {
                        break None;
                    }
                }

                // Partial Order<Open> fill with zero remaining trade liquidity
                OrderFill::Partial => {
                    // Partial-fill means trade quantity is all the remaining trade liquidity
                    let trade_quantity = remaining_liquidity;

                    // Generate execution Trade from partial Order<Open> fill
                    best_bid.state.filled_quantity += trade_quantity;
                    trades.push(self.generate_trade(
                        best_bid.clone(),
                        trade_quantity,
                        fees_percent,
                    ));

                    break Some(best_bid);
                }
            }
        };

        // If remaining best bid had a partial-fill, or is not a match, put it back as the best bid
        if let Some(remaining_best_bid) = remaining_best_bid {
            self.bids.push(remaining_best_bid);
        }

        trades
    }

    /// Generate a client [`Trade`] with a unique [`TradeId`] for this [`Instrument`] market.
    pub fn generate_trade(
        &self,
        order: Order<Open>,
        trade_quantity: f64,
        fees_percent: f64,
    ) -> Trade {
        // Calculate the trade fees (denominated in base or quote depending on Order Side)
        let fees = calculate_fees(&order, trade_quantity, fees_percent);

        // Generate execution Trade from the Order<Open> match
        Trade {
            id: self.trade_id(),
            order_id: order.state.id,
            instrument: order.instrument,
            side: order.side,
            price: order.state.price,
            quantity: trade_quantity,
            fees,
        }
    }

    /// Use the `trade_counter` value to generate a unique [`TradeId`] for this [`Instrument`]
    /// market.
    pub fn trade_id(&self) -> TradeId {
        TradeId(self.trade_counter.to_string())
    }

    /// Simulates [`Side::Sell`] trades by using the [`PublicTrade`] liquidity to match on open
    /// client bid [`Order<Open>`]s.
    pub fn match_asks(&mut self, trade: &PublicTrade, fees_percent: f64) -> Vec<Trade> {
        // Keep track of how much trade liquidity is remaining to match with
        let mut remaining_liquidity = trade.amount;

        // Collection of execution Trades generated from Order<Open> matches
        let mut trades = vec![];

        let remaining_best_ask = loop {
            // Pop the best Order<Open>
            let mut best_ask = match self.asks.pop() {
                Some(best_ask) => best_ask,
                None => break None,
            };

            // Break with remaining best ask if it's not a match, or trade liquidity is exhausted
            if best_ask.state.price > trade.price || remaining_liquidity <= 0.0 {
                break Some(best_ask);
            }

            // Remaining liquidity is either a full-fill or a partial-fill
            self.trade_counter += 1;
            match OrderFill::kind(&best_ask, remaining_liquidity) {
                // Full Order<Open> fill
                OrderFill::Full => {
                    // Remove trade quantity from remaining liquidity
                    let trade_quantity = best_ask.state.remaining_quantity();
                    remaining_liquidity -= trade_quantity;

                    // Generate execution Trade from full Order<Open> fill
                    trades.push(self.generate_trade(best_ask, trade_quantity, fees_percent));

                    // If exact full fill with zero remaining liquidity (highly unlikely), break
                    if remaining_liquidity == 0.0 {
                        break None;
                    }
                }

                // Partial Order<Open> fill with zero remaining trade liquidity
                OrderFill::Partial => {
                    // Partial-fill means trade quantity is all the remaining trade liquidity
                    let trade_quantity = remaining_liquidity;

                    // Generate execution Trade from partial Order<Open> fill
                    best_ask.state.filled_quantity += trade_quantity;
                    trades.push(self.generate_trade(
                        best_ask.clone(),
                        trade_quantity,
                        fees_percent,
                    ));

                    break Some(best_ask);
                }
            }
        };

        // If remaining best ask had a partial-fill, or is not a match, put it back as the best ask
        if let Some(remaining_best_bid) = remaining_best_ask {
            self.asks.push(remaining_best_bid);
        }

        trades
    }

    /// Calculates the total number of open bids and asks.
    pub fn num_orders(&self) -> usize {
        self.bids.len() + self.asks.len()
    }
}

/// Communicates if an [`Order<Open>`] liquidity match is a full or partial fill. Partial fills
/// leave the order still open with some proportion of the initial quantity still active.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub enum OrderFill {
    Full,
    Partial,
}

impl OrderFill {
    /// Determine the [`OrderFill`] kind given the [`Order<Open>`] and the available liquidity.
    pub fn kind(order: &Order<Open>, liquidity: f64) -> Self {
        match order.state.remaining_quantity() <= liquidity {
            true => Self::Full,
            false => Self::Partial,
        }
    }
}

/// Calculate the [`SymbolFees`] of a [`Order<Open>`] match (trade).
pub fn calculate_fees(order: &Order<Open>, trade_quantity: f64, fees_percent: f64) -> SymbolFees {
    match order.side {
        Side::Buy => SymbolFees::new(order.instrument.base.clone(), fees_percent * trade_quantity),
        Side::Sell => SymbolFees::new(
            order.instrument.quote.clone(),
            fees_percent * order.state.price * trade_quantity,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::ClientOrderId,
        simulated::exchange::account::order::Orders,
        test_util::{client_orders, order_open, public_trade, trade},
    };
    use barter_integration::model::Side;
    use uuid::Uuid;

    #[test]
    fn test_client_orders_has_matching_order() {
        struct TestCase {
            orders: Orders,
            input_trade: PublicTrade,
            expected: Option<Side>,
        }

        let cid = ClientOrderId(Uuid::new_v4());

        let tests = vec![
            TestCase {
                // TC0: No matching bids or asks since no open orders
                orders: client_orders(0, vec![], vec![]),
                input_trade: public_trade(Side::Buy, 100.0, 1.0),
                expected: None,
            },
            TestCase {
                // TC1: No matching bid for trade with no asks open
                orders: client_orders(0, vec![order_open(cid, Side::Buy, 100.0, 1.0, 0.0)], vec![]),
                input_trade: public_trade(Side::Buy, 150.0, 1.0),
                expected: None,
            },
            TestCase {
                // TC2: No matching ask for trade with no bids open
                orders: client_orders(
                    0,
                    vec![],
                    vec![order_open(cid, Side::Sell, 100.0, 1.0, 0.0)],
                ),
                input_trade: public_trade(Side::Sell, 50.0, 1.0),
                expected: None,
            },
            TestCase {
                // TC3: Exact matching bid for trade with no asks open
                orders: client_orders(0, vec![order_open(cid, Side::Buy, 100.0, 1.0, 0.0)], vec![]),
                input_trade: public_trade(Side::Buy, 100.0, 1.0),
                expected: Some(Side::Buy),
            },
            TestCase {
                // TC4: Exact matching ask for trade with no bids open
                orders: client_orders(
                    0,
                    vec![],
                    vec![order_open(cid, Side::Sell, 100.0, 1.0, 0.0)],
                ),
                input_trade: public_trade(Side::Sell, 100.0, 1.0),
                expected: Some(Side::Sell),
            },
            TestCase {
                // TC5: No matches for trade with open bids and asks
                orders: client_orders(
                    0,
                    vec![order_open(cid, Side::Buy, 50.0, 1.0, 0.0)],
                    vec![order_open(cid, Side::Sell, 150.0, 1.0, 0.0)],
                ),
                input_trade: public_trade(Side::Buy, 100.0, 1.0),
                expected: None,
            },
            TestCase {
                // TC6: Trade matches bid & ask (same price), so take larger quantity bid
                orders: client_orders(
                    0,
                    vec![order_open(cid, Side::Buy, 100.0, 100.0, 0.0)],
                    vec![order_open(cid, Side::Sell, 100.0, 1.0, 0.0)],
                ),
                input_trade: public_trade(Side::Buy, 100.0, 1.0),
                expected: Some(Side::Buy),
            },
            TestCase {
                // TC6: Trade matches bid & ask (same price), so take larger quantity ask
                orders: client_orders(
                    0,
                    vec![order_open(cid, Side::Buy, 100.0, 1.0, 0.0)],
                    vec![order_open(cid, Side::Sell, 100.0, 100.0, 0.0)],
                ),
                input_trade: public_trade(Side::Buy, 100.0, 1.0),
                expected: Some(Side::Sell),
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.orders.has_matching_order(&test.input_trade);
            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }

    #[test]
    fn test_client_orders_match_bids() {
        struct TestCase {
            orders: Orders,
            input_trade: PublicTrade,
            input_fees_percent: f64,
            expected_orders: Orders,
            expected_trades: Vec<Trade>,
        }

        let cid = ClientOrderId(Uuid::new_v4());

        let tests = vec![
            TestCase {
                // TC0: Best bid matches the PublicTrade w/ a full-fill
                orders: client_orders(
                    0,
                    vec![
                        order_open(cid, Side::Buy, 100.0, 1.0, 0.0),
                        order_open(cid, Side::Buy, 200.0, 1.0, 0.0),
                    ],
                    vec![],
                ),
                input_trade: public_trade(Side::Buy, 200.0, 1.0),
                input_fees_percent: 0.1,
                expected_orders: client_orders(
                    1,
                    vec![order_open(cid, Side::Buy, 100.0, 1.0, 0.0)],
                    vec![],
                ),
                expected_trades: vec![trade(
                    TradeId(1.to_string()),
                    Side::Buy,
                    200.0,
                    1.0,
                    SymbolFees::new("base", 0.1 * 1.0),
                )],
            },
            TestCase {
                // TC1: Two bids match the PublicTrade w/ two full-fills
                orders: client_orders(
                    0,
                    vec![
                        order_open(cid, Side::Buy, 100.0, 1.0, 0.0),
                        order_open(cid, Side::Buy, 200.0, 1.0, 0.0),
                    ],
                    vec![],
                ),
                input_trade: public_trade(Side::Buy, 100.0, 2.0),
                input_fees_percent: 0.1,
                expected_orders: client_orders(2, vec![], vec![]),
                expected_trades: vec![
                    trade(
                        TradeId(1.to_string()),
                        Side::Buy,
                        200.0,
                        1.0,
                        SymbolFees::new("base", 0.1 * 1.0),
                    ),
                    trade(
                        TradeId(2.to_string()),
                        Side::Buy,
                        100.0,
                        1.0,
                        SymbolFees::new("base", 0.1 * 1.0),
                    ),
                ],
            },
            TestCase {
                // TC2: Two bids match the PublicTrade w/ one full-fill & one partial-fill
                orders: client_orders(
                    0,
                    vec![
                        order_open(cid, Side::Buy, 100.0, 1.0, 0.0),
                        order_open(cid, Side::Buy, 200.0, 1.0, 0.0),
                    ],
                    vec![],
                ),
                input_trade: public_trade(Side::Sell, 100.0, 1.5),
                input_fees_percent: 0.1,
                expected_orders: client_orders(
                    2,
                    vec![order_open(cid, Side::Buy, 100.0, 1.0, 0.5)],
                    vec![],
                ),
                expected_trades: vec![
                    trade(
                        TradeId(1.to_string()),
                        Side::Buy,
                        200.0,
                        1.0,
                        SymbolFees::new("base", 0.1 * 1.0),
                    ),
                    trade(
                        TradeId(2.to_string()),
                        Side::Buy,
                        100.0,
                        0.5,
                        SymbolFees::new("base", 0.1 * 0.5),
                    ),
                ],
            },
            TestCase {
                // TC3: No bids match the PublicTrade
                orders: client_orders(
                    0,
                    vec![
                        order_open(cid, Side::Buy, 100.0, 1.0, 0.0),
                        order_open(cid, Side::Buy, 200.0, 1.0, 0.0),
                    ],
                    vec![],
                ),
                input_trade: public_trade(Side::Sell, 1_000_000_000.0, 1.0),
                input_fees_percent: 0.1,
                expected_orders: client_orders(
                    0,
                    vec![
                        order_open(cid, Side::Buy, 100.0, 1.0, 0.0),
                        order_open(cid, Side::Buy, 200.0, 1.0, 0.0),
                    ],
                    vec![],
                ),
                expected_trades: vec![],
            },
        ];

        for (index, mut test) in tests.into_iter().enumerate() {
            let actual_trades = test
                .orders
                .match_bids(&test.input_trade, test.input_fees_percent);
            assert_eq!(actual_trades, test.expected_trades, "TC{}", index);

            let actual_orders = test.orders;
            assert_eq!(actual_orders, test.expected_orders, "TC{}", index);
        }
    }

    #[test]
    fn test_client_orders_match_asks() {
        struct TestCase {
            orders: Orders,
            input_trade: PublicTrade,
            input_fees_percent: f64,
            expected_orders: Orders,
            expected_trades: Vec<Trade>,
        }

        let cid = ClientOrderId(Uuid::new_v4());

        let tests = vec![
            TestCase {
                // TC0: Best ask matches the PublicTrade w/ a full-fill
                orders: client_orders(
                    0,
                    vec![],
                    vec![
                        order_open(cid, Side::Sell, 200.0, 1.0, 0.0),
                        order_open(cid, Side::Sell, 100.0, 1.0, 0.0),
                    ],
                ),
                input_trade: public_trade(Side::Buy, 100.0, 1.0),
                input_fees_percent: 0.1,
                expected_orders: client_orders(
                    1,
                    vec![],
                    vec![order_open(cid, Side::Sell, 200.0, 1.0, 0.0)],
                ),
                expected_trades: vec![trade(
                    TradeId(1.to_string()),
                    Side::Sell,
                    100.0,
                    1.0,
                    SymbolFees::new("quote", 0.1 * 100.0 * 1.0),
                )],
            },
            TestCase {
                // TC1: Two asks match the PublicTrade w/ two full-fills
                orders: client_orders(
                    0,
                    vec![],
                    vec![
                        order_open(cid, Side::Sell, 200.0, 1.0, 0.0),
                        order_open(cid, Side::Sell, 100.0, 1.0, 0.0),
                    ],
                ),
                input_trade: public_trade(Side::Buy, 200.0, 2.0),
                input_fees_percent: 0.1,
                expected_orders: client_orders(2, vec![], vec![]),
                expected_trades: vec![
                    trade(
                        TradeId(1.to_string()),
                        Side::Sell,
                        100.0,
                        1.0,
                        SymbolFees::new("quote", 0.1 * 100.0 * 1.0),
                    ),
                    trade(
                        TradeId(2.to_string()),
                        Side::Sell,
                        200.0,
                        1.0,
                        SymbolFees::new("quote", 0.1 * 200.0 * 1.0),
                    ),
                ],
            },
            TestCase {
                // TC2: Two asks match the PublicTrade w/ one full-fill & one partial-fill
                orders: client_orders(
                    0,
                    vec![],
                    vec![
                        order_open(cid, Side::Sell, 200.0, 1.0, 0.0),
                        order_open(cid, Side::Sell, 100.0, 1.0, 0.0),
                    ],
                ),
                input_trade: public_trade(Side::Sell, 200.0, 1.5),
                input_fees_percent: 0.1,
                expected_orders: client_orders(
                    2,
                    vec![],
                    vec![order_open(cid, Side::Sell, 200.0, 1.0, 0.5)],
                ),
                expected_trades: vec![
                    trade(
                        TradeId(1.to_string()),
                        Side::Sell,
                        100.0,
                        1.0,
                        SymbolFees::new("quote", 0.1 * 100.0 * 1.0),
                    ),
                    trade(
                        TradeId(2.to_string()),
                        Side::Sell,
                        200.0,
                        0.5,
                        SymbolFees::new("quote", 0.1 * 200.0 * 0.5),
                    ),
                ],
            },
            TestCase {
                // TC3: No asks match the PublicTrade
                orders: client_orders(
                    0,
                    vec![],
                    vec![
                        order_open(cid, Side::Sell, 200.0, 1.0, 0.0),
                        order_open(cid, Side::Sell, 100.0, 1.0, 0.0),
                    ],
                ),
                input_trade: public_trade(Side::Sell, 1.0, 1.0),
                input_fees_percent: 0.1,
                expected_orders: client_orders(
                    0,
                    vec![],
                    vec![
                        order_open(cid, Side::Sell, 200.0, 1.0, 0.0),
                        order_open(cid, Side::Sell, 100.0, 1.0, 0.0),
                    ],
                ),
                expected_trades: vec![],
            },
        ];

        for (index, mut test) in tests.into_iter().enumerate() {
            let actual_trades = test
                .orders
                .match_asks(&test.input_trade, test.input_fees_percent);
            assert_eq!(actual_trades, test.expected_trades, "TC{}", index);

            let actual_orders = test.orders;
            assert_eq!(actual_orders, test.expected_orders, "TC{}", index);
        }
    }

    #[test]
    fn test_client_orders_num_orders() {
        struct TestCase {
            orders: Orders,
            expected_num: usize,
        }

        let cid = ClientOrderId(Uuid::new_v4());

        let tests = vec![
            TestCase {
                // TC0: Empty orders
                orders: client_orders(0, vec![], vec![]),
                expected_num: 0,
            },
            TestCase {
                // TC1: one bid, empty ask
                orders: client_orders(0, vec![order_open(cid, Side::Buy, 150.0, 1.0, 0.0)], vec![]),
                expected_num: 1,
            },
            TestCase {
                // TC2: empty bids, one ask
                orders: client_orders(
                    0,
                    vec![],
                    vec![order_open(cid, Side::Sell, 150.0, 1.0, 0.0)],
                ),
                expected_num: 1,
            },
            TestCase {
                // TC2: many of each
                orders: client_orders(
                    0,
                    vec![
                        order_open(cid, Side::Sell, 150.0, 1.0, 0.0),
                        order_open(cid, Side::Sell, 150.0, 1.0, 0.0),
                    ],
                    vec![
                        order_open(cid, Side::Sell, 150.0, 1.0, 0.0),
                        order_open(cid, Side::Sell, 150.0, 1.0, 0.0),
                    ],
                ),
                expected_num: 4,
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = test.orders.num_orders();
            assert_eq!(actual, test.expected_num, "TC{} failed", index);
        }
    }

    #[test]
    fn test_order_fill_kind() {
        struct TestCase {
            input_order: Order<Open>,
            input_liquidity: f64,
            expected: OrderFill,
        }

        let cid = ClientOrderId(Uuid::new_v4());

        let tests = vec![
            TestCase {
                // TC0: Zero filled bid is fully filled by remaining liquidity
                input_order: order_open(cid, Side::Buy, 10.0, 10.0, 0.0),
                input_liquidity: 10.0,
                expected: OrderFill::Full,
            },
            TestCase {
                // TC1: Partially filled bid is fully filled by remaining liquidity
                input_order: order_open(cid, Side::Buy, 10.0, 10.0, 5.0),
                input_liquidity: 10.0,
                expected: OrderFill::Full,
            },
            TestCase {
                // TC2: Zero filled bid is partially filled by remaining liquidity
                input_order: order_open(cid, Side::Buy, 10.0, 10.0, 0.0),
                input_liquidity: 5.0,
                expected: OrderFill::Partial,
            },
            TestCase {
                // TC3: Partially filled bid is partially filled by remaining liquidity
                input_order: order_open(cid, Side::Buy, 10.0, 10.0, 1.0),
                input_liquidity: 5.0,
                expected: OrderFill::Partial,
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = OrderFill::kind(&test.input_order, test.input_liquidity);
            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }

    #[test]
    fn test_calculate_fees() {
        struct TestCase {
            order: Order<Open>,
            trade_quantity: f64,
            fees_percent: f64,
            expected: SymbolFees,
        }

        let cid = ClientOrderId(Uuid::new_v4());

        let tests = vec![
            TestCase {
                // TC0: 10% trade fees from matched Side::Buy order
                order: order_open(cid, Side::Buy, 100.0, 10.0, 0.0),
                trade_quantity: 10.0,
                fees_percent: 0.1,
                expected: SymbolFees::new("base", 0.1 * 10.0),
            },
            TestCase {
                // TC1: 50% trade fees from matched Side::Sell order
                order: order_open(cid, Side::Sell, 100.0, 10.0, 0.0),
                trade_quantity: 10.0,
                fees_percent: 0.5,
                expected: SymbolFees::new("quote", 0.5 * 100.0 * 10.0),
            },
        ];

        for (index, test) in tests.into_iter().enumerate() {
            let actual = calculate_fees(&test.order, test.trade_quantity, test.fees_percent);
            assert_eq!(actual, test.expected, "TC{} failed", index);
        }
    }
}

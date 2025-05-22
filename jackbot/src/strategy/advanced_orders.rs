use jackbot_execution::order::{
    Order, OrderKey, OrderKind, TimeInForce,
    id::{ClientOrderId, OrderId, StrategyId},
    request::{OrderRequestCancel, OrderRequestOpen, RequestCancel, RequestOpen},
    state::{ActiveOrderState, Open},
};
use jackbot_instrument::{Side, asset::AssetIndex, exchange::ExchangeIndex, instrument::InstrumentIndex};
use rust_decimal::Decimal;
use rand::Rng;
use chrono::Utc;

use crate::engine::state::{EngineState, instrument::{filter::InstrumentFilter, data::InstrumentDataState}, order::Orders};

/// Generate TWAP (time-weighted average price) order slice quantities with randomised weights.
/// The returned quantities will sum to `total_quantity`.
pub fn twap_slices<R: Rng>(total_quantity: Decimal, slices: usize, randomness: f64, rng: &mut R) -> Vec<Decimal> {
    assert!(slices > 0);
    let mut weights: Vec<f64> = (0..slices).map(|_| 1.0 + rng.gen_range(-randomness..=randomness)).collect();
    let sum: f64 = weights.iter().sum();
    weights.iter_mut().for_each(|w| *w /= sum);
    let mut quantities: Vec<Decimal> = weights
        .iter()
        .map(|w| total_quantity * Decimal::from_f64(*w).unwrap())
        .collect();
    let diff: Decimal = total_quantity - quantities.iter().copied().sum::<Decimal>();
    if let Some(last) = quantities.last_mut() {
        *last += diff;
    }
    quantities
}

/// Generate VWAP (volume-weighted average price) order slice quantities with randomised weights.
/// The provided `volumes` slice defines relative volume weights for each slice.
/// The returned quantities will sum to `total_quantity`.
pub fn vwap_slices<R: Rng>(total_quantity: Decimal, volumes: &[Decimal], randomness: f64, rng: &mut R) -> Vec<Decimal> {
    assert!(!volumes.is_empty());
    let total_volume: Decimal = volumes.iter().copied().sum();
    let mut weights: Vec<f64> = volumes
        .iter()
        .map(|v| (v / total_volume).to_f64().unwrap())
        .collect();
    weights.iter_mut().for_each(|w| *w *= 1.0 + rng.gen_range(-randomness..=randomness));
    let sum: f64 = weights.iter().sum();
    weights.iter_mut().for_each(|w| *w /= sum);
    let mut quantities: Vec<Decimal> = weights
        .iter()
        .map(|w| total_quantity * Decimal::from_f64(*w).unwrap())
        .collect();
    let diff = total_quantity - quantities.iter().copied().sum::<Decimal>();
    if let Some(last) = quantities.last_mut() {
        *last += diff;
    }
    quantities
}

/// Helper trait for accessing best bid and ask prices from instrument data.
pub trait BestBidAsk {
    fn best_bid(&self) -> Option<Decimal>;
    fn best_ask(&self) -> Option<Decimal>;
}

impl BestBidAsk for crate::engine::state::instrument::data::DefaultInstrumentMarketData {
    fn best_bid(&self) -> Option<Decimal> {
        self.l1.bids().levels().first().map(|l| l.price)
    }
    fn best_ask(&self) -> Option<Decimal> {
        self.l1.asks().levels().first().map(|l| l.price)
    }
}

/// Simple "always maker" strategy.
/// Places a post-only limit order at the top of book and reposts if price changes.
#[derive(Debug, Clone)]
pub struct AlwaysMakerStrategy {
    pub id: StrategyId,
    pub side: Side,
    pub quantity: Decimal,
}

impl<GlobalData, InstrumentData> crate::strategy::algo::AlgoStrategy<ExchangeIndex, InstrumentIndex>
    for AlwaysMakerStrategy
where
    InstrumentData: InstrumentDataState<ExchangeIndex, AssetIndex, InstrumentIndex> + BestBidAsk,
{
    type State = EngineState<GlobalData, InstrumentData>;

    fn generate_algo_orders(
        &self,
        state: &Self::State,
    ) -> (
        impl IntoIterator<Item = OrderRequestCancel<ExchangeIndex, InstrumentIndex>>,
        impl IntoIterator<Item = OrderRequestOpen<ExchangeIndex, InstrumentIndex>>,
    ) {
        let mut cancels = Vec::new();
        let mut opens = Vec::new();
        for instr in state.instruments.instruments(&InstrumentFilter::None) {
            let price = match self.side {
                Side::Buy => instr.data.best_bid(),
                Side::Sell => instr.data.best_ask(),
            };
            let Some(price) = price else { continue };
            let existing = instr.orders.orders().next();
            if let Some(order) = existing {
                if order.price != price {
                    cancels.push(OrderRequestCancel {
                        key: order.key.clone(),
                        state: RequestCancel { id: None },
                    });
                    opens.push(OrderRequestOpen {
                        key: OrderKey {
                            exchange: order.key.exchange.clone(),
                            instrument: order.key.instrument.clone(),
                            strategy: self.id.clone(),
                            cid: ClientOrderId::random(),
                        },
                        state: RequestOpen {
                            side: self.side,
                            price,
                            quantity: self.quantity,
                            kind: OrderKind::Limit,
                            time_in_force: TimeInForce::GoodUntilCancelled { post_only: true },
                        },
                    });
                }
            } else {
                opens.push(OrderRequestOpen {
                    key: OrderKey {
                        exchange: instr.instrument.exchange,
                        instrument: instr.key,
                        strategy: self.id.clone(),
                        cid: ClientOrderId::random(),
                    },
                    state: RequestOpen {
                        side: self.side,
                        price,
                        quantity: self.quantity,
                        kind: OrderKind::Limit,
                        time_in_force: TimeInForce::GoodUntilCancelled { post_only: true },
                    },
                });
            }
        }
        (cancels, opens)
    }
}

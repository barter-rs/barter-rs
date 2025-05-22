use crate::{
    client::ExecutionClient,
    order::{
        request::{OrderRequestOpen, RequestOpen},
        Order,
        state::Open,
    },
    error::UnindexedOrderError,
};
use jackbot_instrument::{
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use jackbot_data::books::aggregator::OrderBookAggregator;
use rand::prelude::*;
use rust_decimal::Decimal;
use tokio::time::{sleep, Duration};

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

#[derive(Debug, Clone)]
pub struct VwapScheduler<C, R>
where
    C: ExecutionClient + Clone,
    R: Rng + Clone,
{
    pub client: C,
    pub aggregator: OrderBookAggregator,
    rng: R,
}

impl<C, R> VwapScheduler<C, R>
where
    C: ExecutionClient + Clone,
    R: Rng + Clone,
{
    pub fn new(client: C, aggregator: OrderBookAggregator, rng: R) -> Self {
        Self { client, aggregator, rng }
    }

    fn generate_delays(&mut self, volumes: &[Decimal], base: Duration) -> Vec<Duration> {
        let spread = if let (Some((_, bid)), Some((_, ask))) = (self.aggregator.best_bid(), self.aggregator.best_ask()) {
            (ask - bid).abs()
        } else {
            Decimal::ONE
        };
        let factor = spread.to_f64().unwrap_or(1.0);
        let mut weights: Vec<f64> = volumes.iter().map(|v| v.to_f64().unwrap()).collect();
        let sum: f64 = weights.iter().sum();
        weights.iter_mut().for_each(|w| *w /= sum);
        weights
            .iter()
            .map(|w| {
                let jitter = self.rng.gen_range(0.0..base.as_millis() as f64 * factor);
                base.mul_f64(*w) + Duration::from_millis(jitter as u64)
            })
            .collect()
    }

    pub async fn execute(
        &mut self,
        request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
        volumes: &[Decimal],
        randomness: f64,
        base_delay: Duration,
    ) -> Vec<Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>> {
        let quantities = vwap_slices(request.state.quantity, volumes, randomness, &mut self.rng);
        let delays = self.generate_delays(volumes, base_delay);
        let mut results = Vec::with_capacity(quantities.len());
        for (qty, delay) in quantities.into_iter().zip(delays.into_iter()) {
            sleep(delay).await;
            let req = OrderRequestOpen {
                key: request.key.clone(),
                state: RequestOpen {
                    side: request.state.side,
                    price: request.state.price,
                    quantity: qty,
                    kind: request.state.kind,
                    time_in_force: request.state.time_in_force,
                },
            };
            let res = self.client.clone().open_order(req).await;
            results.push(res);
        }
        results
    }
}


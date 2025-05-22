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
use crate::advanced::OrderExecutionStrategy;
use async_trait::async_trait;

/// Generate TWAP (time-weighted average price) order slice quantities with randomised weights.
/// The returned quantities will sum to `total_quantity`.
pub fn twap_slices<R: Rng>(total_quantity: Decimal, slices: usize, randomness: f64, rng: &mut R) -> Vec<Decimal> {
    assert!(slices > 0);
    let mut weights: Vec<f64> = (0..slices)
        .map(|_| 1.0 + rng.gen_range(-randomness..=randomness))
        .collect();
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

/// TWAP scheduler that slices an order into parts and schedules them over time.
#[derive(Debug, Clone)]
pub struct TwapScheduler<C, R>
where
    C: ExecutionClient + Clone,
    R: Rng + Clone,
{
    pub client: C,
    pub aggregator: OrderBookAggregator,
    rng: R,
}

/// Parameters controlling TWAP execution behaviour.
#[derive(Debug, Clone, Copy)]
pub struct TwapConfig {
    /// Number of order slices to generate.
    pub slices: usize,
    /// Randomness applied to slice weighting.
    pub randomness: f64,
    /// Base delay between order slices.
    pub base_delay: Duration,
}

impl<C, R> TwapScheduler<C, R>
where
    C: ExecutionClient + Clone,
    R: Rng + Clone,
{
    pub fn new(client: C, aggregator: OrderBookAggregator, rng: R) -> Self {
        Self { client, aggregator, rng }
    }

    fn generate_delays(&mut self, slices: usize, base: Duration) -> Vec<Duration> {
        let spread = if let (Some((_, bid)), Some((_, ask))) = (self.aggregator.best_bid(), self.aggregator.best_ask()) {
            (ask - bid).abs()
        } else {
            Decimal::ONE
        };
        let factor = spread.to_f64().unwrap_or(1.0);
        (0..slices)
            .map(|_| {
                let jitter = self.rng.gen_range(0.0..base.as_millis() as f64 * factor);
                base + Duration::from_millis(jitter as u64)
            })
            .collect()
    }

    /// Execute the provided order request using a TWAP schedule.
    pub async fn execute(
        &mut self,
        request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
        slices: usize,
        randomness: f64,
        base_delay: Duration,
    ) -> Vec<Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>> {
        let quantities = twap_slices(request.state.quantity, slices, randomness, &mut self.rng);
        let delays = self.generate_delays(slices, base_delay);
        let mut results = Vec::with_capacity(slices);
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

#[async_trait]
impl<C, R> OrderExecutionStrategy for TwapScheduler<C, R>
where
    C: ExecutionClient + Clone + Send + Sync,
    R: Rng + Clone + Send + Sync,
{
    type Config = TwapConfig;

    async fn execute(
        &mut self,
        request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
        config: Self::Config,
    ) -> Vec<Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>> {
        TwapScheduler::execute(
            self,
            request,
            config.slices,
            config.randomness,
            config.base_delay,
        )
        .await
    }
}


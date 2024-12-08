use crate::v2::{
    balance::AssetBalance,
    execution::{error::UnindexedClientError, UnindexedAccountEvent, UnindexedAccountSnapshot},
    order::{Cancelled, Open, Order, RequestCancel, RequestOpen},
};
use barter_instrument::{
    asset::name::AssetNameExchange, exchange::ExchangeId, instrument::name::InstrumentNameExchange,
};
use derive_more::Constructor;
use futures::{stream::BoxStream, Stream};
use std::{collections::HashSet, future::Future};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tracing::error;

pub trait ExecutionClient
where
    Self: Clone,
{
    const EXCHANGE: ExchangeId;

    type Config: Clone;
    type AccountStream: Stream<Item = UnindexedAccountEvent>;

    fn new(config: Self::Config) -> Self;

    fn account_snapshot(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> impl Future<Output = Result<UnindexedAccountSnapshot, UnindexedClientError>> + Send;

    fn account_stream(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> impl Future<Output = Result<Self::AccountStream, UnindexedClientError>> + Send;

    fn cancel_order(
        &self,
        request: Order<ExchangeId, &InstrumentNameExchange, RequestCancel>,
    ) -> impl Future<
        Output = Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedClientError>>,
    > + Send;

    fn cancel_orders<'a>(
        &self,
        requests: impl IntoIterator<Item = Order<ExchangeId, &'a InstrumentNameExchange, RequestCancel>>,
    ) -> impl Stream<
        Item = Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedClientError>>,
    > {
        futures::stream::FuturesUnordered::from_iter(
            requests
                .into_iter()
                .map(|request| self.cancel_order(request)),
        )
    }

    fn open_order(
        &self,
        request: Order<ExchangeId, &InstrumentNameExchange, RequestOpen>,
    ) -> impl Future<
        Output = Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedClientError>>,
    > + Send;

    fn open_orders<'a>(
        &self,
        requests: impl IntoIterator<Item = Order<ExchangeId, &'a InstrumentNameExchange, RequestOpen>>,
    ) -> impl Stream<Item = Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedClientError>>>
    {
        futures::stream::FuturesUnordered::from_iter(
            requests.into_iter().map(|request| self.open_order(request)),
        )
    }

    fn fetch_balances(
        &self,
    ) -> impl Future<Output = Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError>>;

    fn fetch_open_orders(
        &self,
    ) -> impl Future<
        Output = Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError>,
    >;
}

#[derive(Debug, Clone, Constructor)]
pub struct MockExecution {
    pub mocked_exchange: ExchangeId,
    pub state: UnindexedAccountSnapshot,
    pub account_stream_tx: tokio::sync::broadcast::Sender<UnindexedAccountEvent>,
}

#[derive(Debug, Clone, Constructor)]
pub struct MockExecutionConfig {
    pub mocked_exchange: ExchangeId,
    pub initial_state: UnindexedAccountSnapshot,
    pub fees_percent: f64,
}

impl ExecutionClient for MockExecution {
    const EXCHANGE: ExchangeId = ExchangeId::Mock;
    type Config = MockExecutionConfig;
    type AccountStream = BoxStream<'static, UnindexedAccountEvent>;

    fn new(config: Self::Config) -> Self {
        const ACCOUNT_STREAM_CAPACITY: usize = 256;

        let (tx, _rx) = tokio::sync::broadcast::channel(ACCOUNT_STREAM_CAPACITY);

        // Sanity check: AccountSnapshot Orders are for mocked ExchangeId
        config
            .initial_state
            .instruments
            .iter()
            .for_each(|instrument| {
                instrument.orders.iter().for_each(|order| {
                    // Check Order is for the mocked ExchangeId
                    if order.exchange != config.mocked_exchange {
                        panic!(
                            "MockExecution initial AccountSnapshot contains Order with: \
                            {}, but Self is configured to mock: {}. Order: {:?}",
                            order.exchange, config.mocked_exchange, order
                        )
                    }
                })
            });

        Self {
            mocked_exchange: config.mocked_exchange,
            state: config.initial_state,
            account_stream_tx: tx,
        }
    }

    async fn account_snapshot(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> Result<UnindexedAccountSnapshot, UnindexedClientError> {
        // Sanity check (not performance critical):
        self.check_for_untracked_assets_and_instruments(assets, instruments)
            .map_err(|(untracked_assets, untracked_instruments)| {
                UnindexedClientError::AccountSnapshot(format!(
                    "MockExecution not configured for assets: {:?} and instruments: {:?}",
                    untracked_assets, untracked_instruments,
                ))
            })?;

        Ok(self.state.clone())
    }

    async fn account_stream(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> Result<Self::AccountStream, UnindexedClientError> {
        // Sanity check (not performance critical):
        self.check_for_untracked_assets_and_instruments(assets, instruments)
            .map_err(|(untracked_assets, untracked_instruments)| {
                UnindexedClientError::AccountStream(format!(
                    "MockExecution not configured for assets: {:?} and instruments: {:?}",
                    untracked_assets, untracked_instruments,
                ))
            })?;

        Ok(futures::StreamExt::boxed(
            BroadcastStream::new(self.account_stream_tx.subscribe()).map_while(
                |result| match result {
                    Ok(event) => Some(event),
                    Err(error) => {
                        error!(
                            ?error,
                            "MockExecution Broadcast AccountStream lagged - terminating"
                        );
                        None
                    }
                },
            ),
        ))
    }

    async fn cancel_order(
        &self,
        _request: Order<ExchangeId, &InstrumentNameExchange, RequestCancel>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedClientError>> {
        // Todo: It's possible we want a MockExchange which this ExecutionClient interacts with...
        //   that would more easily allow simulating real open & cancel async behaviour...
        //   -> at minimum probably want some more optimised data structures such as
        //    Order hashmap, or perhaps AssetStates & InstrumentStates
        todo!()
    }

    async fn open_order(
        &self,
        _request: Order<ExchangeId, &InstrumentNameExchange, RequestOpen>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedClientError>> {
        todo!()
    }

    async fn fetch_balances(
        &self,
    ) -> Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError> {
        Ok(self.state.balances.clone())
    }

    async fn fetch_open_orders(
        &self,
    ) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError> {
        Ok(self
            .state
            .instruments
            .iter()
            .flat_map(|state| state.orders.iter().filter_map(Order::as_open))
            .collect())
    }
}

impl MockExecution {
    fn check_for_untracked_assets_and_instruments<'a>(
        &self,
        assets: &'a [AssetNameExchange],
        instruments: &'a [InstrumentNameExchange],
    ) -> Result<
        (),
        (
            HashSet<&'a AssetNameExchange>,
            HashSet<&'a InstrumentNameExchange>,
        ),
    > {
        let mut assets = assets.iter().collect::<HashSet<_>>();
        for tracked_asset in self.state.assets() {
            assets.remove(tracked_asset);
        }

        let mut instruments = instruments.iter().collect::<HashSet<_>>();
        for tracked_instrument in self.state.instruments() {
            instruments.remove(tracked_instrument);
        }

        if assets.is_empty() && instruments.is_empty() {
            Ok(())
        } else {
            Err((assets, instruments))
        }
    }
}

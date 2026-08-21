use crate::{
    UnindexedAccountEvent, UnindexedAccountSnapshot,
    balance::AssetBalance,
    error::{UnindexedClientError, UnindexedOrderError},
    order::{
        Order, UnindexedOrderSnapshot,
        request::{
            OrderRequestCancel, OrderRequestOpen, OrderRequestSnapshot,
            UnindexedOrderResponseCancel,
        },
        state::Open,
    },
    trade::Trade,
};
use barter_instrument::{
    asset::{QuoteAsset, name::AssetNameExchange},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use chrono::{DateTime, Utc};
use futures::{FutureExt, Stream, future::BoxFuture};
use std::future::Future;

mod binance;
pub mod mock;

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
        request: OrderRequestCancel<ExchangeId, &InstrumentNameExchange>,
    ) -> impl Future<Output = Option<UnindexedOrderResponseCancel>> + Send;

    fn cancel_orders<'a>(
        &self,
        requests: impl IntoIterator<Item = OrderRequestCancel<ExchangeId, &'a InstrumentNameExchange>>,
    ) -> impl Stream<Item = Option<UnindexedOrderResponseCancel>> {
        futures::stream::FuturesUnordered::from_iter(
            requests
                .into_iter()
                .map(|request| self.cancel_order(request)),
        )
    }

    fn open_order(
        &self,
        request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
    ) -> impl Future<
        Output = Option<
            Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>,
        >,
    > + Send;

    fn open_orders<'a>(
        &self,
        requests: impl IntoIterator<Item = OrderRequestOpen<ExchangeId, &'a InstrumentNameExchange>>,
    ) -> impl Stream<
        Item = Option<Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>>,
    > {
        futures::stream::FuturesUnordered::from_iter(
            requests.into_iter().map(|request| self.open_order(request)),
        )
    }

    fn fetch_order_snapshot(
        &self,
        request: OrderRequestSnapshot<ExchangeId, InstrumentNameExchange>,
    ) -> impl Future<Output = Result<UnindexedOrderSnapshot, UnindexedClientError>> + Send;

    /// Futures should be returned in the same order as the requests.
    fn fetch_order_snapshots(
        &self,
        requests: impl IntoIterator<Item = OrderRequestSnapshot<ExchangeId, InstrumentNameExchange>>,
    ) -> impl Iterator<Item = BoxFuture<'_, Result<UnindexedOrderSnapshot, UnindexedClientError>>>
    where
        Self: Send,
    {
        let client = self.clone();
        requests.into_iter().map(move |request| {
            let client = client.clone();
            async move { client.fetch_order_snapshot(request).await }.boxed()
        })
    }

    fn fetch_balances(
        &self,
        assets: &[AssetNameExchange],
    ) -> impl Future<Output = Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError>>;

    fn fetch_trades(
        &self,
        time_since: DateTime<Utc>,
    ) -> impl Future<Output = Result<Vec<Trade<QuoteAsset, InstrumentNameExchange>>, UnindexedClientError>>;
}

use crate::v2::{
    balance::AssetBalance,
    execution::{error::ExchangeExecutionError, IndexedAccountEvent, IndexedAccountSnapshot},
    order::{Cancelled, Open, Order, RequestCancel, RequestOpen},
    position::Position,
};
use barter_instrument::{
    asset::name::AssetNameExchange, exchange::ExchangeId, instrument::name::InstrumentNameExchange,
};
use derive_more::Constructor;
use futures::Stream;
use std::future::Future;

pub trait ExecutionClient
where
    Self: Clone,
{
    const EXCHANGE: ExchangeId;

    type Config: Clone;
    type AccountStream: Stream<Item = IndexedAccountEvent>;

    fn new(config: Self::Config) -> Self;

    fn account_snapshot<'a>(
        &self,
        assets: impl Iterator<Item = &'a AssetNameExchange>,
        instruments: impl Iterator<Item = &'a InstrumentNameExchange>,
    ) -> impl Future<Output = Result<IndexedAccountSnapshot, ExchangeExecutionError>>;

    fn account_stream(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> impl Future<Output = Result<Self::AccountStream, ExchangeExecutionError>> + Send;

    fn cancel_order<ExchangeKey: Send>(
        &self,
        request: Order<ExchangeKey, &InstrumentNameExchange, RequestCancel>,
    ) -> impl Future<
        Output = Order<
            ExchangeKey,
            InstrumentNameExchange,
            Result<Cancelled, ExchangeExecutionError>,
        >,
    > + Send;

    fn open_order<ExchangeKey: Send>(
        &self,
        request: Order<ExchangeKey, &InstrumentNameExchange, RequestOpen>,
    ) -> impl Future<
        Output = Order<ExchangeKey, InstrumentNameExchange, Result<Open, ExchangeExecutionError>>,
    > + Send;

    fn fetch_balances(
        &self,
    ) -> impl Future<Output = Result<Vec<AssetBalance<AssetNameExchange>>, ExchangeExecutionError>>;

    fn fetch_positions(
        &self,
    ) -> impl Future<Output = Result<Vec<Position<InstrumentNameExchange>>, ExchangeExecutionError>>;

    fn fetch_open_orders(
        &self,
    ) -> impl Future<
        Output = Result<
            Vec<Order<ExchangeId, InstrumentNameExchange, Open>>,
            ExchangeExecutionError,
        >,
    >;
}

#[derive(Debug, Clone, Constructor)]
pub struct MockExecution;

#[derive(Debug, Clone, Constructor)]
pub struct MockExecutionConfig;

impl ExecutionClient for MockExecution {
    const EXCHANGE: ExchangeId = ExchangeId::Mock;
    type Config = MockExecutionConfig; // Todo: AccountSnapshot
    type AccountStream = futures::stream::Empty<IndexedAccountEvent>;

    fn new(_config: Self::Config) -> Self {
        Self
    }

    async fn account_snapshot<'a>(
        &self,
        _assets: impl Iterator<Item = &'a AssetNameExchange>,
        _instruments: impl Iterator<Item = &'a InstrumentNameExchange>,
    ) -> Result<IndexedAccountSnapshot, ExchangeExecutionError> {
        todo!()
    }

    async fn account_stream(
        &self,
        _assets: &[AssetNameExchange],
        _instruments: &[InstrumentNameExchange],
    ) -> Result<Self::AccountStream, ExchangeExecutionError> {
        todo!()
    }

    async fn cancel_order<ExchangeKey: Send>(
        &self,
        _request: Order<ExchangeKey, &InstrumentNameExchange, RequestCancel>,
    ) -> Order<ExchangeKey, InstrumentNameExchange, Result<Cancelled, ExchangeExecutionError>> {
        todo!()
    }

    async fn open_order<ExchangeKey: Send>(
        &self,
        _request: Order<ExchangeKey, &InstrumentNameExchange, RequestOpen>,
    ) -> Order<ExchangeKey, InstrumentNameExchange, Result<Open, ExchangeExecutionError>> {
        todo!()
    }

    async fn fetch_balances(
        &self,
    ) -> Result<Vec<AssetBalance<AssetNameExchange>>, ExchangeExecutionError> {
        todo!()
    }

    async fn fetch_positions(
        &self,
    ) -> Result<Vec<Position<InstrumentNameExchange>>, ExchangeExecutionError> {
        todo!()
    }

    async fn fetch_open_orders(
        &self,
    ) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, ExchangeExecutionError> {
        todo!()
    }
}

use crate::{
    client::ExecutionClient,
    UnindexedAccountEvent, UnindexedAccountSnapshot,
    balance::AssetBalance,
    error::{UnindexedClientError, UnindexedOrderError},
    order::{
        Order,
        request::{OrderRequestCancel, OrderRequestOpen, UnindexedOrderResponseCancel},
        state::Open,
    },
    trade::Trade,
};
use jackbot_instrument::{
    asset::{QuoteAsset, name::AssetNameExchange},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use chrono::{DateTime, Utc};
use futures::{Stream, stream};
use std::future::Future;

#[derive(Debug, Clone, Default)]
pub struct MexcClient;

#[derive(Debug, Clone, Default)]
pub struct MexcConfig;

impl ExecutionClient for MexcClient {
    const EXCHANGE: ExchangeId = ExchangeId::Mexc;
    type Config = MexcConfig;
    type AccountStream = stream::Empty<UnindexedAccountEvent>;

    fn new(_config: Self::Config) -> Self {
        Self
    }

    fn account_snapshot(
        &self,
        _assets: &[AssetNameExchange],
        _instruments: &[InstrumentNameExchange],
    ) -> impl Future<Output = Result<UnindexedAccountSnapshot, UnindexedClientError>> + Send {
        async { unimplemented!("MEXC account_snapshot") }
    }

    fn account_stream(
        &self,
        _assets: &[AssetNameExchange],
        _instruments: &[InstrumentNameExchange],
    ) -> impl Future<Output = Result<Self::AccountStream, UnindexedClientError>> + Send {
        async { Ok(stream::empty()) }
    }

    fn cancel_order(
        &self,
        _request: OrderRequestCancel<ExchangeId, &InstrumentNameExchange>,
    ) -> impl Future<Output = UnindexedOrderResponseCancel> + Send {
        async { unimplemented!("MEXC cancel_order") }
    }

    fn open_order(
        &self,
        _request: OrderRequestOpen<ExchangeId, &InstrumentNameExchange>,
    ) -> impl Future<Output = Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>> + Send {
        async { unimplemented!("MEXC open_order") }
    }

    fn fetch_balances(&self) -> impl Future<Output = Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError>> + Send {
        async { unimplemented!("MEXC fetch_balances") }
    }

    fn fetch_open_orders(
        &self,
    ) -> impl Future<Output = Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError>> + Send {
        async { unimplemented!("MEXC fetch_open_orders") }
    }

    fn fetch_trades(
        &self,
        _time_since: DateTime<Utc>,
    ) -> impl Future<Output = Result<Vec<Trade<QuoteAsset, InstrumentNameExchange>>, UnindexedClientError>> + Send {
        async { unimplemented!("MEXC fetch_trades") }
    }
}


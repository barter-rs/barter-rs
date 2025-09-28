use crate::{
    UnindexedAccountSnapshot,
    balance::AssetBalance,
    error::UnindexedOrderError,
    order::{
        Order,
        request::{OrderRequestCancel, OrderRequestOpen, UnindexedOrderResponseCancel},
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
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct MockExchangeRequest {
    pub time_request: DateTime<Utc>,
    pub kind: MockExchangeRequestKind,
}

impl MockExchangeRequest {
    pub fn new(time_request: DateTime<Utc>, kind: MockExchangeRequestKind) -> Self {
        Self { time_request, kind }
    }

    pub fn fetch_account_snapshot(
        time_request: DateTime<Utc>,
        response_tx: oneshot::Sender<UnindexedAccountSnapshot>,
    ) -> Self {
        Self::new(
            time_request,
            MockExchangeRequestKind::FetchAccountSnapshot { response_tx },
        )
    }

    pub fn fetch_balances(
        time_request: DateTime<Utc>,
        assets: Vec<AssetNameExchange>,
        response_tx: oneshot::Sender<Vec<AssetBalance<AssetNameExchange>>>,
    ) -> Self {
        Self::new(
            time_request,
            MockExchangeRequestKind::FetchBalances {
                response_tx,
                assets,
            },
        )
    }

    pub fn fetch_orders_open(
        time_request: DateTime<Utc>,
        instruments: Vec<InstrumentNameExchange>,
        response_tx: oneshot::Sender<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>>,
    ) -> Self {
        Self::new(
            time_request,
            MockExchangeRequestKind::FetchOrdersOpen {
                response_tx,
                instruments,
            },
        )
    }

    pub fn fetch_trades(
        time_request: DateTime<Utc>,
        response_tx: oneshot::Sender<Vec<Trade<QuoteAsset, InstrumentNameExchange>>>,
        time_since: DateTime<Utc>,
    ) -> Self {
        Self::new(
            time_request,
            MockExchangeRequestKind::FetchTrades {
                response_tx,
                time_since,
            },
        )
    }

    pub fn cancel_order(
        time_request: DateTime<Utc>,
        response_tx: oneshot::Sender<UnindexedOrderResponseCancel>,
        request: OrderRequestCancel<ExchangeId, InstrumentNameExchange>,
    ) -> Self {
        Self::new(
            time_request,
            MockExchangeRequestKind::CancelOrder {
                response_tx,
                request,
            },
        )
    }

    pub fn open_order(
        time_request: DateTime<Utc>,
        response_tx: oneshot::Sender<
            Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>,
        >,
        request: OrderRequestOpen<ExchangeId, InstrumentNameExchange>,
    ) -> Self {
        Self::new(
            time_request,
            MockExchangeRequestKind::OpenOrder {
                response_tx,
                request,
            },
        )
    }
}

#[derive(Debug)]
pub enum MockExchangeRequestKind {
    FetchAccountSnapshot {
        response_tx: oneshot::Sender<UnindexedAccountSnapshot>,
    },
    FetchBalances {
        assets: Vec<AssetNameExchange>,
        response_tx: oneshot::Sender<Vec<AssetBalance<AssetNameExchange>>>,
    },
    FetchOrdersOpen {
        instruments: Vec<InstrumentNameExchange>,
        response_tx: oneshot::Sender<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>>,
    },
    FetchTrades {
        response_tx: oneshot::Sender<Vec<Trade<QuoteAsset, InstrumentNameExchange>>>,
        time_since: DateTime<Utc>,
    },
    CancelOrder {
        response_tx: oneshot::Sender<UnindexedOrderResponseCancel>,
        request: OrderRequestCancel<ExchangeId, InstrumentNameExchange>,
    },
    OpenOrder {
        response_tx: oneshot::Sender<
            Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>>,
        >,
        request: OrderRequestOpen<ExchangeId, InstrumentNameExchange>,
    },
}

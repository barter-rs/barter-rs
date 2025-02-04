use crate::{
    balance::AssetBalance,
    error::{
        ApiError, ClientError, KeyError, OrderError, UnindexedApiError, UnindexedClientError,
        UnindexedOrderError,
    },
    map::ExecutionInstrumentMap,
    order::{
        state::{InactiveOrderState, OrderState, UnindexedOrderState},
        Order, UnindexedOrder,
    },
    trade::Trade,
    AccountEvent, AccountEventKind, AccountSnapshot, InstrumentAccountSnapshot,
    UnindexedAccountEvent, UnindexedAccountSnapshot,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, AssetIndex, QuoteAsset},
    exchange::{ExchangeId, ExchangeIndex},
    index::error::IndexError,
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use barter_integration::{
    snapshot::Snapshot,
    stream::indexed::{IndexedStream, Indexer},
};
use derive_more::Constructor;
use std::sync::Arc;

pub type IndexedAccountStream<St> = IndexedStream<AccountEventIndexer, St>;

#[derive(Debug, Clone, Constructor)]
pub struct AccountEventIndexer {
    pub map: Arc<ExecutionInstrumentMap>,
}

impl Indexer for AccountEventIndexer {
    type Unindexed = UnindexedAccountEvent;
    type Indexed = AccountEvent;

    fn index(&self, item: Self::Unindexed) -> Result<Self::Indexed, IndexError> {
        self.account_event(item)
    }
}

impl AccountEventIndexer {
    pub fn account_event(&self, event: UnindexedAccountEvent) -> Result<AccountEvent, IndexError> {
        let UnindexedAccountEvent { exchange, kind } = event;

        let exchange = self.map.find_exchange_index(exchange)?;

        let kind = match kind {
            AccountEventKind::Snapshot(snapshot) => {
                AccountEventKind::Snapshot(self.snapshot(snapshot)?)
            }
            AccountEventKind::BalanceSnapshot(snapshot) => {
                AccountEventKind::BalanceSnapshot(self.asset_balance(snapshot.0).map(Snapshot)?)
            }
            AccountEventKind::OrderSnapshot(snapshot) => {
                AccountEventKind::OrderSnapshot(self.order_snapshot(snapshot.0).map(Snapshot)?)
            }
            AccountEventKind::Trade(trade) => AccountEventKind::Trade(self.trade(trade)?),
        };

        Ok(AccountEvent { exchange, kind })
    }

    pub fn snapshot(
        &self,
        snapshot: UnindexedAccountSnapshot,
    ) -> Result<AccountSnapshot, IndexError> {
        let UnindexedAccountSnapshot {
            exchange,
            balances,
            instruments,
        } = snapshot;

        let exchange = self.map.find_exchange_index(exchange)?;

        let balances = balances
            .into_iter()
            .map(|balance| self.asset_balance(balance))
            .collect::<Result<Vec<_>, _>>()?;

        let instruments = instruments
            .into_iter()
            .map(|snapshot| {
                let InstrumentAccountSnapshot { instrument, orders } = snapshot;

                let instrument = self.map.find_instrument_index(&instrument)?;

                let orders = orders
                    .into_iter()
                    .map(|order| self.order_snapshot(order))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(InstrumentAccountSnapshot { instrument, orders })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AccountSnapshot {
            exchange,
            balances,
            instruments,
        })
    }

    pub fn asset_balance(
        &self,
        balance: AssetBalance<AssetNameExchange>,
    ) -> Result<AssetBalance<AssetIndex>, IndexError> {
        let AssetBalance {
            asset,
            balance,
            time_exchange,
        } = balance;
        let asset = self.map.find_asset_index(&asset)?;

        Ok(AssetBalance {
            asset,
            balance,
            time_exchange,
        })
    }

    pub fn order_snapshot(&self, order: UnindexedOrder) -> Result<Order, IndexError> {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = order;

        let state = match state {
            UnindexedOrderState::Active(active) => OrderState::Active(active),
            UnindexedOrderState::Inactive(inactive) => match inactive {
                InactiveOrderState::Failed(failed) => match failed {
                    OrderError::Rejected(rejected) => {
                        OrderState::inactive(OrderError::Rejected(self.api_error(rejected)?))
                    }
                    OrderError::Connectivity(error) => {
                        OrderState::inactive(OrderError::Connectivity(error))
                    }
                },
                InactiveOrderState::Cancelled(cancelled) => OrderState::inactive(cancelled),
                InactiveOrderState::FullyFilled => OrderState::fully_filled(),
                InactiveOrderState::Expired => OrderState::expired(),
            },
        };

        Ok(Order {
            exchange: self.map.find_exchange_index(exchange)?,
            instrument: self.map.find_instrument_index(&instrument)?,
            strategy,
            cid,
            side,
            state,
        })
    }

    pub fn api_error(&self, error: UnindexedApiError) -> Result<ApiError, IndexError> {
        Ok(match error {
            UnindexedApiError::Unauthorized(reason) => ApiError::Unauthorized(reason),
            UnindexedApiError::RateLimit => ApiError::RateLimit,
            UnindexedApiError::AssetInvalid(asset, value) => {
                ApiError::AssetInvalid(self.map.find_asset_index(&asset)?, value)
            }
            UnindexedApiError::InstrumentInvalid(instrument, value) => {
                ApiError::InstrumentInvalid(self.map.find_instrument_index(&instrument)?, value)
            }
            UnindexedApiError::BalanceInsufficient(asset, value) => {
                ApiError::BalanceInsufficient(self.map.find_asset_index(&asset)?, value)
            }
            UnindexedApiError::OrderRejected(reason) => ApiError::OrderRejected(reason),
            UnindexedApiError::OrderAlreadyCancelled => ApiError::OrderAlreadyCancelled,
            UnindexedApiError::OrderAlreadyFullyFilled => ApiError::OrderAlreadyFullyFilled,
            UnindexedApiError::OrderNotFound => ApiError::OrderNotFound,
            UnindexedApiError::Custom(reason) => ApiError::Custom(reason),
        })
    }

    pub fn order_request<Kind>(
        &self,
        order: &Order<ExchangeIndex, InstrumentIndex, Kind>,
    ) -> Result<Order<ExchangeId, &InstrumentNameExchange, Kind>, KeyError>
    where
        Kind: Clone,
    {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = order;

        let exchange = self.map.find_exchange_id(*exchange)?;
        let instrument = self.map.find_instrument_name_exchange(*instrument)?;

        Ok(Order {
            exchange,
            instrument,
            strategy: strategy.clone(),
            cid: cid.clone(),
            side: *side,
            state: state.clone(),
        })
    }

    pub fn order_response<Kind>(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, Result<Kind, UnindexedOrderError>>,
    ) -> Result<Order<ExchangeIndex, InstrumentIndex, Result<Kind, OrderError>>, IndexError> {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = order;

        let exchange_index = self.map.find_exchange_index(exchange)?;
        let instrument_index = self.map.find_instrument_index(&instrument)?;
        let state = match state {
            Ok(state) => Ok(state),
            Err(error) => Err(self.order_error(error)?),
        };

        Ok(Order {
            exchange: exchange_index,
            instrument: instrument_index,
            strategy,
            cid,
            side,
            state,
        })
    }

    pub fn order_error(&self, error: UnindexedOrderError) -> Result<OrderError, IndexError> {
        Ok(match error {
            UnindexedOrderError::Connectivity(error) => OrderError::Connectivity(error),
            UnindexedOrderError::Rejected(error) => OrderError::Rejected(self.api_error(error)?),
        })
    }

    pub fn client_error(&self, error: UnindexedClientError) -> Result<ClientError, IndexError> {
        Ok(match error {
            UnindexedClientError::Connectivity(error) => ClientError::Connectivity(error),
            UnindexedClientError::Api(error) => ClientError::Api(self.api_error(error)?),
            UnindexedClientError::AccountSnapshot(value) => ClientError::AccountSnapshot(value),
            UnindexedClientError::AccountStream(value) => ClientError::AccountStream(value),
        })
    }

    pub fn trade(
        &self,
        trade: Trade<QuoteAsset, InstrumentNameExchange>,
    ) -> Result<Trade<QuoteAsset, InstrumentIndex>, IndexError> {
        let Trade {
            id,
            order_id,
            instrument,
            strategy,
            time_exchange,
            side,
            price,
            quantity,
            fees,
        } = trade;

        let instrument_index = self.map.find_instrument_index(&instrument)?;

        Ok(Trade {
            id,
            order_id,
            instrument: instrument_index,
            strategy,
            time_exchange,
            side,
            price,
            quantity,
            fees,
        })
    }
}

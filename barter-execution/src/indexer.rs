use crate::{
    balance::AssetBalance,
    error::{
        IndexedApiError, IndexedClientError, KeyError, UnindexedApiError, UnindexedClientError,
    },
    map::ExecutionInstrumentMap,
    order::{ExchangeOrderState, Order},
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
            AccountEventKind::OrderOpened(order) => {
                AccountEventKind::OrderOpened(self.order_response(order)?)
            }
            AccountEventKind::OrderCancelled(order) => {
                AccountEventKind::OrderCancelled(self.order_response(order)?)
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
                    .map(|order| self.order_open(order))
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

    pub fn order_snapshot(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, ExchangeOrderState>,
    ) -> Result<Order<ExchangeIndex, InstrumentIndex, ExchangeOrderState>, IndexError> {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = order;

        Ok(Order {
            exchange: self.map.find_exchange_index(exchange)?,
            instrument: self.map.find_instrument_index(&instrument)?,
            strategy,
            cid,
            side,
            state,
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

    pub fn order_open(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, ExchangeOrderState>,
    ) -> Result<Order<ExchangeIndex, InstrumentIndex, ExchangeOrderState>, IndexError> {
        let Order {
            exchange,
            instrument,
            strategy,
            cid,
            side,
            state,
        } = order;

        Ok(Order {
            exchange: self.map.find_exchange_index(exchange)?,
            instrument: self.map.find_instrument_index(&instrument)?,
            strategy,
            cid,
            side,
            state,
        })
    }

    pub fn order_response<Kind>(
        &self,
        order: Order<ExchangeId, InstrumentNameExchange, Result<Kind, UnindexedClientError>>,
    ) -> Result<Order<ExchangeIndex, InstrumentIndex, Result<Kind, IndexedClientError>>, IndexError>
    {
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
            Err(error) => Err(self.client_error(error)?),
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

    pub fn client_error(
        &self,
        error: UnindexedClientError,
    ) -> Result<IndexedClientError, IndexError> {
        Ok(match error {
            UnindexedClientError::Connectivity(error) => IndexedClientError::Connectivity(error),
            UnindexedClientError::Api(error) => IndexedClientError::Api(match error {
                UnindexedApiError::RateLimit => IndexedApiError::RateLimit,
                UnindexedApiError::AssetInvalid(asset, value) => {
                    IndexedApiError::AssetInvalid(self.map.find_asset_index(&asset)?, value)
                }
                UnindexedApiError::InstrumentInvalid(instrument, value) => {
                    IndexedApiError::InstrumentInvalid(
                        self.map.find_instrument_index(&instrument)?,
                        value,
                    )
                }
                UnindexedApiError::BalanceInsufficient(asset, value) => {
                    IndexedApiError::BalanceInsufficient(self.map.find_asset_index(&asset)?, value)
                }
                UnindexedApiError::OrderRejected(reason) => IndexedApiError::OrderRejected(reason),
                UnindexedApiError::OrderAlreadyCancelled => IndexedApiError::OrderAlreadyCancelled,
                UnindexedApiError::OrderAlreadyFullyFilled => {
                    IndexedApiError::OrderAlreadyFullyFilled
                }
            }),
            UnindexedClientError::AccountSnapshot(value) => {
                IndexedClientError::AccountSnapshot(value)
            }
            UnindexedClientError::AccountStream(value) => IndexedClientError::AccountStream(value),
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

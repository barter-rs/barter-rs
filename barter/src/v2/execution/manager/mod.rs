use crate::v2::{
    execution::{
        error::{ExchangeApiError, ExchangeExecutionError, IndexedApiError, IndexedExecutionError},
        manager::client::ExecutionClient,
        map::ExecutionInstrumentMap,
        AccountEvent, AccountEventKind, ExecutionRequest,
    },
    order::{Order, RequestCancel, RequestOpen},
};
use barter_data::streams::{
    consumer::StreamKey,
    reconnect,
    reconnect::stream::{init_reconnecting_stream, ReconnectingStream, ReconnectionBackoffPolicy},
};
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    instrument::{name::InstrumentNameExchange, InstrumentIndex},
};
use barter_integration::{
    channel::{mpsc_unbounded, Tx, UnboundedTx},
    stream::merge::merge,
};
use derive_more::Constructor;
use futures::{future::try_join, Stream, StreamExt};
use tracing::info;

pub mod client;

pub type AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey> =
    reconnect::Event<ExchangeId, AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>;

pub type ExecutionInitResult<Manager, AccountStream> = Result<
    (
        AccountEvent<ExchangeIndex, AssetIndex, InstrumentIndex>,
        Manager,
        AccountStream,
    ),
    IndexedExecutionError,
>;

/// Per exchange execution manager.
#[derive(Debug, Constructor)]
pub struct ExecutionManager<RequestStream, Client> {
    pub request_stream: RequestStream,
    pub response_tx: UnboundedTx<AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>>,
    pub client: Client,
    pub instruments: ExecutionInstrumentMap,
}

impl<RequestStream, Client> ExecutionManager<RequestStream, Client>
where
    RequestStream: Stream<Item = ExecutionRequest<ExchangeIndex, InstrumentIndex>> + Unpin,
    Client: ExecutionClient,
{
    pub async fn init(
        request_stream: RequestStream,
        client: Client,
        instrument_map: ExecutionInstrumentMap,
        reconnect_policy: ReconnectionBackoffPolicy,
    ) -> ExecutionInitResult<
        Self,
        impl Stream<Item = AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>>,
    > {
        // Determine ExchangeId associated with this ExecutionClient
        let exchange = Client::EXCHANGE;

        // Determine StreamKey for use in logging
        let stream_key = StreamKey::new_general("account_stream", exchange);

        info!(
            %exchange,
            policy = ?reconnect_policy,
            ?stream_key,
            "AccountStream with auto reconnect initialising"
        );

        // Future for fetching AccountSnapshot
        let account_snapshot_future = client.account_snapshot(
            instrument_map.assets.iter(),
            instrument_map.instruments.iter(),
        );

        // Future for reconnecting AccountEvent Stream
        let client_clone = client.clone();
        let assets = instrument_map
            .exchange_assets()
            .cloned()
            .collect::<Vec<_>>();
        let instruments = instrument_map
            .exchange_instruments()
            .cloned()
            .collect::<Vec<_>>();
        let account_stream_future = init_reconnecting_stream(move || {
            let client = client_clone.clone();
            let assets = assets.clone();
            let instruments = instruments.clone();
            async move { client.account_stream(&assets, &instruments).await }
        });

        // Await Futures
        let (snapshot, stream) = try_join(account_snapshot_future, account_stream_future)
            .await
            .map_err(|error| map_to_indexed_execution_error(&instrument_map, error, None))?;

        // Construct channel to communicate ExecutionRequest responses to Engine ie/ AccountEvents
        let (response_tx, response_rx) = mpsc_unbounded();

        // Construct merged AccountEvent Stream
        let account_stream = merge(
            response_rx.into_stream(),
            stream
                .with_reconnect_backoff(reconnect_policy, stream_key)
                .with_reconnection_events(exchange),
        );

        Ok((
            AccountEvent::new(instrument_map.exchange, snapshot),
            Self::new(request_stream, response_tx, client.clone(), instrument_map),
            account_stream,
        ))
    }

    pub async fn run(mut self) {
        while let Some(request) = self.request_stream.next().await {
            match request {
                ExecutionRequest::Cancel(request) => {
                    let response = self.cancel_order(request).await;
                    if self.response_tx.send(response).is_err() {
                        break;
                    }
                }
                ExecutionRequest::Open(request) => {
                    let response = self.open_order(request).await;
                    if self.response_tx.send(response).is_err() {
                        break;
                    }
                }
            }
        }
    }

    pub async fn cancel_order(
        &self,
        request: Order<ExchangeIndex, InstrumentIndex, RequestCancel>,
    ) -> AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex> {
        let request = map_order_to_instrument_name_exchange(&self.instruments, request);

        let response = self.client.cancel_order(request).await;

        AccountStreamEvent::Item(AccountEvent {
            exchange: self.instruments.exchange,
            kind: AccountEventKind::OrderCancelled(map_order_to_instrument_index(
                &self.instruments,
                response,
            )),
        })
    }

    pub async fn open_order(
        &self,
        request: Order<ExchangeIndex, InstrumentIndex, RequestOpen>,
    ) -> AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex> {
        let request = map_order_to_instrument_name_exchange(&self.instruments, request);

        let response = self.client.open_order(request).await;

        AccountStreamEvent::Item(AccountEvent {
            exchange: self.instruments.exchange,
            kind: AccountEventKind::OrderOpened(map_order_to_instrument_index(
                &self.instruments,
                response,
            )),
        })
    }
}

// Todo: Do we need to map ExchangeKey?
pub fn map_order_to_instrument_name_exchange<ExchangeKey, State>(
    instruments: &ExecutionInstrumentMap,
    order: Order<ExchangeKey, InstrumentIndex, State>,
) -> Order<ExchangeKey, &InstrumentNameExchange, State> {
    let Order {
        exchange,
        instrument,
        cid,
        side,
        state,
    } = order;

    Order {
        exchange,
        instrument: instruments.find_instrument_name_exchange(instrument),
        cid,
        side,
        state,
    }
}

// Todo: Probably need to map ExchangeKey?
fn map_order_to_instrument_index<ExchangeKey, State>(
    instruments: &ExecutionInstrumentMap,
    order: Order<ExchangeKey, InstrumentNameExchange, Result<State, ExchangeExecutionError>>,
) -> Order<ExchangeKey, InstrumentIndex, Result<State, IndexedExecutionError>> {
    let Order {
        exchange,
        instrument,
        cid,
        side,
        state,
    } = order;

    let instrument_index = instruments.find_instrument_index(&instrument);

    Order {
        exchange,
        instrument: instrument_index,
        cid,
        side,
        state: state.map_err(|error| {
            map_to_indexed_execution_error(instruments, error, Some(instrument_index))
        }),
    }
}

fn map_to_indexed_execution_error(
    instruments: &ExecutionInstrumentMap,
    error: ExchangeExecutionError,
    instrument_index: Option<InstrumentIndex>,
) -> IndexedExecutionError {
    match error {
        ExchangeExecutionError::Connectivity(error) => IndexedExecutionError::Connectivity(error),
        ExchangeExecutionError::ApiError(error) => IndexedExecutionError::ApiError(match error {
            ExchangeApiError::RateLimit => IndexedApiError::RateLimit,
            ExchangeApiError::InstrumentInvalid(instrument, value) => {
                IndexedApiError::InstrumentInvalid(
                    instrument_index
                        .unwrap_or_else(|| instruments.find_instrument_index(&instrument)),
                    value,
                )
            }
            ExchangeApiError::BalanceInsufficient(asset, value) => {
                IndexedApiError::BalanceInsufficient(instruments.find_asset_index(&asset), value)
            }
            ExchangeApiError::OrderRejected(cid) => IndexedApiError::OrderRejected(cid),
            ExchangeApiError::OrderAlreadyCancelled(cid) => IndexedApiError::OrderRejected(cid),
            ExchangeApiError::OrderAlreadyFullyFilled(cid) => IndexedApiError::OrderRejected(cid),
        }),
    }
}
